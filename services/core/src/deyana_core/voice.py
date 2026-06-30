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
    VoiceOption,
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


@dataclass(frozen=True)
class VoiceCatalog:
    voices: tuple[VoiceOption, ...] = ()


class LocalVoiceService:
    def __init__(self, data_dir: Path) -> None:
        self.data_dir = data_dir
        self.settings_path = data_dir / "voice-settings.json"
        self._voice_catalog: VoiceCatalog | None = None

    def read_settings(self) -> VoiceSettings:
        defaults = self.default_settings()
        try:
            data = json.loads(self.settings_path.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            return defaults

        try:
            settings = VoiceSettings.model_validate({**defaults.model_dump(), **data})
        except ValidationError:
            return defaults

        return self.resolve_voice(settings)
    def patch_settings(self, patch: VoiceSettingsPatch) -> VoiceSettings:
        settings = self.read_settings()
        updates = patch.model_dump(exclude_unset=True)
        if "tts_voice" in updates:
            updates["tts_voice"] = self.validate_voice_selection(updates["tts_voice"])
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
        catalog = self.voice_catalog()
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
        elif settings.enabled and settings.tts_enabled and not settings.tts_voice:
            tts_status = "missing"
            detail = "No installed female text-to-speech voice is available."

        return VoiceStatusResponse(
            enabled=settings.enabled,
            muted=settings.muted,
            tts_enabled=settings.tts_enabled,
            stt_status=stt_status,
            tts_status=tts_status,
            stt_engine=settings.stt_engine,
            tts_engine=settings.tts_engine,
            language=settings.language,
            active_tts_voice=settings.tts_voice,
            available_tts_voices=list(catalog.voices),
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
        if not settings.tts_voice:
            raise VoiceUnavailableError("No installed female text-to-speech voice is available.")

    def default_settings(self) -> VoiceSettings:
        return VoiceSettings(
            tts_voice=self.preferred_female_voice(self.voice_catalog()),
            updated_at=utc_timestamp(),
        )

    def voice_catalog(self) -> VoiceCatalog:
        if self._voice_catalog is None:
            self._voice_catalog = discover_windows_voice_catalog()
        return self._voice_catalog

    def resolve_voice(self, settings: VoiceSettings) -> VoiceSettings:
        catalog = self.voice_catalog()
        if settings.tts_voice:
            canonical_name = canonical_voice_name(settings.tts_voice, catalog)
            if canonical_name:
                return settings.model_copy(update={"tts_voice": canonical_name})
        return settings.model_copy(update={"tts_voice": self.preferred_female_voice(catalog)})

    def validate_voice_selection(self, requested_voice: str | None) -> str | None:
        catalog = self.voice_catalog()
        if not requested_voice or not requested_voice.strip():
            return self.preferred_female_voice(catalog)

        canonical_name = canonical_voice_name(requested_voice, catalog)
        if canonical_name:
            return canonical_name
        raise VoiceUnavailableError(
            f"The female local voice '{requested_voice}' is not installed or is not selectable."
        )

    @staticmethod
    def preferred_female_voice(catalog: VoiceCatalog) -> str | None:
        female_voices = [voice for voice in catalog.voices if voice.gender == "female"]
        zira = next((voice for voice in female_voices if "zira" in voice.name.casefold()), None)
        if zira:
            return zira.name

        english_female = next(
            (voice for voice in female_voices if voice.language.casefold().startswith("en")),
            None,
        )
        if english_female:
            return english_female.name
        return female_voices[0].name if female_voices else None


def canonical_voice_name(requested_voice: str, catalog: VoiceCatalog) -> str | None:
    normalized = requested_voice.strip().casefold()
    return next((voice.name for voice in catalog.voices if voice.name.casefold() == normalized), None)


def discover_windows_voice_catalog() -> VoiceCatalog:
    if platform.system().lower() != "windows" or not powershell_path():
        return VoiceCatalog()

    script = r"""
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  $defaultVoice = $speaker.Voice.Name
  $voices = @(
    $speaker.GetInstalledVoices() |
      Where-Object { $_.Enabled } |
      ForEach-Object {
        [PSCustomObject]@{
          name = $_.VoiceInfo.Name
          gender = $_.VoiceInfo.Gender.ToString()
          language = $_.VoiceInfo.Culture.Name
          isSystemDefault = ($_.VoiceInfo.Name -eq $defaultVoice)
        }
      }
  )
  [PSCustomObject]@{
    systemDefault = $defaultVoice
    voices = $voices
  } | ConvertTo-Json -Depth 4 -Compress
} finally {
  $speaker.Dispose()
}
"""
    result = run_powershell(script, {}, timeout=10)
    if result.returncode != 0 or not result.stdout.strip():
        return VoiceCatalog()

    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError:
        return VoiceCatalog()

    raw_voices = payload.get("voices", []) if isinstance(payload, dict) else []
    if isinstance(raw_voices, dict):
        raw_voices = [raw_voices]

    voices: list[VoiceOption] = []
    for raw_voice in raw_voices:
        if not isinstance(raw_voice, dict):
            continue
        name = str(raw_voice.get("name", "")).strip()
        if not name:
            continue
        raw_gender = str(raw_voice.get("gender", "unknown")).strip().casefold()
        if raw_gender != "female":
            continue
        voices.append(
            VoiceOption(
                name=name,
                gender="female",
                language=str(raw_voice.get("language", "")).strip() or "unknown",
                is_system_default=bool(raw_voice.get("isSystemDefault", False)),
            )
        )

    return VoiceCatalog(voices=tuple(voices))


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
