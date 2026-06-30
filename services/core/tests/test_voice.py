from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.models import VoiceOption
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings
from deyana_core.voice import (
    CommandResult,
    LocalVoiceService,
    VoiceCatalog,
)


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def windows_voice_catalog() -> VoiceCatalog:
    return VoiceCatalog(
        voices=(
            VoiceOption(
                name="Microsoft Zira Desktop",
                gender="female",
                language="en-US",
                is_system_default=True,
            ),
            VoiceOption(
                name="Microsoft Hazel Desktop",
                gender="female",
                language="en-GB",
                is_system_default=False,
            ),
        ),
    )


def test_voice_settings_default_to_muted_local_female_voice(tmp_path, monkeypatch) -> None:
    monkeypatch.setattr(
        "deyana_core.voice.discover_windows_voice_catalog",
        windows_voice_catalog,
    )

    with make_client(tmp_path) as client:
        settings = client.get("/voice/settings")
        status = client.get("/voice/status")
        transcribe = client.post("/voice/transcribe", json={})

    assert settings.status_code == 200
    assert settings.json()["enabled"] is False
    assert settings.json()["muted"] is True
    assert settings.json()["ttsEnabled"] is False
    assert settings.json()["ttsVoice"] == "Microsoft Zira Desktop"
    assert settings.json()["transcriptRetention"] == "none"
    assert status.status_code == 200
    assert status.json()["rawAudioStored"] is False
    assert status.json()["sttStatus"] == "disabled"
    assert status.json()["activeTtsVoice"] == "Microsoft Zira Desktop"
    assert [voice["name"] for voice in status.json()["availableTtsVoices"]] == [
        "Microsoft Zira Desktop",
        "Microsoft Hazel Desktop",
    ]
    assert {voice["gender"] for voice in status.json()["availableTtsVoices"]} == {"female"}
    assert transcribe.status_code == 400
    assert "disabled" in transcribe.json()["detail"].lower()


def test_voice_transcribes_with_local_provider_without_storing_audio(tmp_path, monkeypatch) -> None:
    monkeypatch.setattr(LocalVoiceService, "provider_status", lambda _self: "available")
    monkeypatch.setattr(
        "deyana_core.voice.run_windows_stt",
        lambda _language, _duration: CommandResult(0, "Open the launch checklist.\n", ""),
    )

    with make_client(tmp_path) as client:
        client.patch("/voice/settings", json={"enabled": True, "muted": False, "listenSeconds": 4})
        response = client.post("/voice/transcribe", json={})

    assert response.status_code == 200
    assert response.json()["transcript"] == "Open the launch checklist."
    assert response.json()["engine"] == "windows_speech"
    assert response.json()["durationSeconds"] == 4
    assert response.json()["rawAudioStored"] is False


def test_voice_tts_uses_only_selected_local_female_voice(tmp_path, monkeypatch) -> None:
    spoken: list[tuple[str, str | None]] = []
    monkeypatch.setattr(LocalVoiceService, "provider_status", lambda _self: "available")
    monkeypatch.setattr(
        "deyana_core.voice.discover_windows_voice_catalog",
        windows_voice_catalog,
    )
    monkeypatch.setattr(
        "deyana_core.voice.run_windows_tts",
        lambda text, voice, rate, volume: spoken.append((text, voice)) or CommandResult(0, "", ""),
    )

    with make_client(tmp_path) as client:
        blocked = client.post("/voice/speak", json={"text": "Local only."})
        client.patch(
            "/voice/settings",
            json={
                "enabled": True,
                "muted": True,
                "ttsEnabled": True,
                "ttsVoice": "Microsoft Hazel Desktop",
                "ttsRate": 1,
                "ttsVolume": 70,
            },
        )
        selected = client.get("/voice/settings")
        response = client.post("/voice/speak", json={"text": "Local only."})

    assert blocked.status_code == 400
    assert response.status_code == 200
    assert response.json()["spoken"] is True
    assert response.json()["characters"] == len("Local only.")
    assert response.json()["rawAudioStored"] is False
    assert selected.json()["ttsVoice"] == "Microsoft Hazel Desktop"
    assert spoken == [("Local only.", "Microsoft Hazel Desktop")]