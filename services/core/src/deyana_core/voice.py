from __future__ import annotations

import json
import os
import platform
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from pydantic import ValidationError

from .models import (
    VoiceSettings,
    VoiceSettingsPatch,
    VoiceSpeakRequest,
    VoiceSpeakResponse,
    VoiceStatusResponse,
    VoiceTranscriptRequest,
    VoiceTranscriptResponse,
)
from .runtime_time import utc_timestamp


class VoiceUnavailableError(RuntimeError):
    pass


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    stdout: str
    stderr: str


class LocalVoiceService:
    def __init__(self, data_dir: Path) -> None:
        self.data_dir = data_dir
        self.settings_path = data_dir / "voice-settings.json"

    def read_settings(self) -> VoiceSettings:
        try:
            data = json.loads(self.settings_path.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            return self.default_settings()

        try:
            return VoiceSettings.model_validate({**self.default_settings().model_dump(), **data})
        except ValidationError:
            return self.default_settings()

    def patch_settings(self, patch: VoiceSettingsPatch) -> VoiceSettings:
        settings = self.read_settings()
        updates = patch.model_dump(exclude_unset=True)
        next_settings = settings.model_copy(update={**updates, "updated_at": utc_timestamp()})
        self.write_settings(next_settings)
        return next_settings

    def write_settings(self, settings: VoiceSettings) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        temp_path = self.settings_path.with_suffix(".json.tmp")
        temp_path.write_text(json.dumps(settings.model_dump(mode="json", by_alias=True), indent=2), encoding="utf-8")
        temp_path.replace(self.settings_path)

    def status(self) -> VoiceStatusResponse:
        settings = self.read_settings()
        provider_status = self.provider_status()
        stt_status = provider_status
        tts_status = provider_status
        detail = "Windows local speech APIs are available."

        if not settings.enabled:
            stt_status = "disabled"
            tts_status = "disabled"
            detail = "Voice is disabled until the user enables it."
        elif settings.muted:
            stt_status = "muted"
            detail = "Microphone input is muted."
        elif provider_status != "available":
            detail = "No supported local speech engine is available on this machine."

        if settings.enabled and not settings.tts_enabled:
            tts_status = "disabled"

        return VoiceStatusResponse(
            enabled=settings.enabled,
            muted=settings.muted,
            tts_enabled=settings.tts_enabled,
            stt_status=stt_status,
            tts_status=tts_status,
            stt_engine=settings.stt_engine,
            tts_engine=settings.tts_engine,
            language=settings.language,
            raw_audio_stored=False,
            detail=detail,
            checked_at=utc_timestamp(),
        )

    def transcribe(self, request: VoiceTranscriptRequest) -> VoiceTranscriptResponse:
        settings = self.read_settings()
        self.require_stt_ready(settings)
        duration = request.listen_seconds or settings.listen_seconds
        result = run_windows_stt(settings.language, duration)
        transcript = result.stdout.strip()
        if result.returncode != 0:
            raise VoiceUnavailableError(result.stderr.strip() or "Local speech recognition failed.")

        return VoiceTranscriptResponse(
            transcript=transcript,
            engine=settings.stt_engine,
            language=settings.language,
            duration_seconds=duration,
            raw_audio_stored=False,
            created_at=utc_timestamp(),
        )

    def speak(self, request: VoiceSpeakRequest) -> VoiceSpeakResponse:
        settings = self.read_settings()
        text = request.text.strip()
        if not text:
            raise ValueError("Speech text is required.")

        self.require_tts_ready(settings)
        result = run_windows_tts(
            text=text,
            voice=settings.tts_voice,
            rate=settings.tts_rate,
            volume=settings.tts_volume,
        )
        if result.returncode != 0:
            raise VoiceUnavailableError(result.stderr.strip() or "Local text-to-speech failed.")

        return VoiceSpeakResponse(
            spoken=True,
            engine=settings.tts_engine,
            characters=len(text),
            raw_audio_stored=False,
            created_at=utc_timestamp(),
        )

    def provider_status(self) -> str:
        if platform.system().lower() != "windows":
            return "unsupported"
        if not powershell_path():
            return "missing"
        return "available"

    def require_stt_ready(self, settings: VoiceSettings) -> None:
        if not settings.enabled:
            raise VoiceUnavailableError("Voice is disabled.")
        if settings.muted:
            raise VoiceUnavailableError("Microphone input is muted.")
        if self.provider_status() != "available":
            raise VoiceUnavailableError("A supported local speech recognition engine is not available.")

    def require_tts_ready(self, settings: VoiceSettings) -> None:
        if not settings.enabled:
            raise VoiceUnavailableError("Voice is disabled.")
        if not settings.tts_enabled:
            raise VoiceUnavailableError("Text-to-speech is disabled.")
        if self.provider_status() != "available":
            raise VoiceUnavailableError("A supported local text-to-speech engine is not available.")

    @staticmethod
    def default_settings() -> VoiceSettings:
        return VoiceSettings(updated_at=utc_timestamp())


def run_windows_stt(language: str, listen_seconds: int) -> CommandResult:
    script = r"""
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$seconds = [Math]::Max(2, [Math]::Min(20, [int]$env:DEYANA_STT_SECONDS))
$cultureName = $env:DEYANA_STT_LANGUAGE
$recognizer = $null
try {
  $culture = [System.Globalization.CultureInfo]::GetCultureInfo($cultureName)
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine($culture)
} catch {
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine
}
try {
  $recognizer.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
  $recognizer.SetInputToDefaultAudioDevice()
  $result = $recognizer.Recognize([TimeSpan]::FromSeconds($seconds))
  if ($null -ne $result) { $result.Text }
} finally {
  if ($null -ne $recognizer) { $recognizer.Dispose() }
}
"""
    return run_powershell(
        script,
        {
            "DEYANA_STT_LANGUAGE": language,
            "DEYANA_STT_SECONDS": str(listen_seconds),
        },
        timeout=max(8, listen_seconds + 8),
    )


def run_windows_tts(*, text: str, voice: str | None, rate: int, volume: int) -> CommandResult:
    script = r"""
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  $speaker.Rate = [int]$env:DEYANA_TTS_RATE
  $speaker.Volume = [int]$env:DEYANA_TTS_VOLUME
  if ($env:DEYANA_TTS_VOICE) {
    $speaker.SelectVoice($env:DEYANA_TTS_VOICE)
  }
  $speaker.Speak($env:DEYANA_TTS_TEXT)
} finally {
  $speaker.Dispose()
}
"""
    return run_powershell(
        script,
        {
            "DEYANA_TTS_TEXT": text,
            "DEYANA_TTS_VOICE": voice or "",
            "DEYANA_TTS_RATE": str(rate),
            "DEYANA_TTS_VOLUME": str(volume),
        },
        timeout=max(10, min(60, len(text) // 12 + 10)),
    )


def run_powershell(script: str, env_patch: dict[str, str], timeout: int) -> CommandResult:
    executable = powershell_path()
    if not executable:
        return CommandResult(returncode=1, stdout="", stderr="PowerShell is required for Windows local voice.")

    env = {**os.environ, **env_patch}
    creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    try:
        completed = subprocess.run(
            [
                executable,
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ],
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
            creationflags=creationflags,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return CommandResult(returncode=1, stdout="", stderr="Local voice command timed out.")

    return CommandResult(
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def powershell_path() -> str | None:
    return shutil.which("powershell.exe") or shutil.which("pwsh.exe") or shutil.which("powershell")
