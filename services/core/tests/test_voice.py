from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings
from deyana_core.voice import CommandResult, LocalVoiceService


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def test_voice_settings_default_to_muted_local_only(tmp_path) -> None:
    with make_client(tmp_path) as client:
        settings = client.get("/voice/settings")
        status = client.get("/voice/status")
        transcribe = client.post("/voice/transcribe", json={})

    assert settings.status_code == 200
    assert settings.json()["enabled"] is False
    assert settings.json()["muted"] is True
    assert settings.json()["ttsEnabled"] is False
    assert settings.json()["transcriptRetention"] == "none"
    assert status.status_code == 200
    assert status.json()["rawAudioStored"] is False
    assert status.json()["sttStatus"] == "disabled"
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


def test_voice_tts_uses_local_provider_and_can_be_muted_independently(tmp_path, monkeypatch) -> None:
    spoken: list[str] = []
    monkeypatch.setattr(LocalVoiceService, "provider_status", lambda _self: "available")
    monkeypatch.setattr(
        "deyana_core.voice.run_windows_tts",
        lambda text, voice, rate, volume: spoken.append(text) or CommandResult(0, "", ""),
    )

    with make_client(tmp_path) as client:
        blocked = client.post("/voice/speak", json={"text": "Local only."})
        client.patch(
            "/voice/settings",
            json={"enabled": True, "muted": True, "ttsEnabled": True, "ttsRate": 1, "ttsVolume": 70},
        )
        response = client.post("/voice/speak", json={"text": "Local only."})

    assert blocked.status_code == 400
    assert response.status_code == 200
    assert response.json()["spoken"] is True
    assert response.json()["characters"] == len("Local only.")
    assert response.json()["rawAudioStored"] is False
    assert spoken == ["Local only."]
