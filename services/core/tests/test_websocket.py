from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def test_websocket_emits_app_ready_event(tmp_path) -> None:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    with TestClient(create_app(RuntimeState(settings))) as client:
        with client.websocket_connect("/ws") as websocket:
            event = websocket.receive_json()

    assert event["type"] == "app.ready"
    assert event["payload"]["service"] == "deyana-core"
    assert event["payload"]["lifecycle"] == "running"
