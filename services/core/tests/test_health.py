from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def test_health_endpoint_reports_running_core(tmp_path) -> None:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    with TestClient(create_app(RuntimeState(settings))) as client:
        response = client.get("/health")

    assert response.status_code == 200
    body = response.json()
    assert body["status"] == "ok"
    assert body["service"] == "deyana-core"
    assert body["lifecycle"] == "running"
    assert body["uptimeSeconds"] >= 0
