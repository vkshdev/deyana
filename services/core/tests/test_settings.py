from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def test_settings_patch_persists_local_preferences(tmp_path) -> None:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")

    with TestClient(create_app(RuntimeState(settings))) as client:
        response = client.patch(
            "/settings",
            json={"privacyMode": "local_only", "modelProfile": "balanced", "syncMode": "low_frequency"},
        )

    assert response.status_code == 200
    patched = response.json()
    assert patched["privacyMode"] == "local_only"
    assert patched["modelProfile"] == "balanced"
    assert patched["syncMode"] == "low_frequency"

    with TestClient(create_app(RuntimeState(settings))) as client:
        persisted = client.get("/settings").json()

    assert persisted["modelProfile"] == "balanced"
    assert persisted["syncMode"] == "low_frequency"
