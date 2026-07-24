from __future__ import annotations

from fastapi.testclient import TestClient
import pytest

from deyana_core.app import create_app
from deyana_core.browser.models import (
    BrowserPersonalityProfile,
    WhatsAppBusyModePolicy,
)
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


@pytest.fixture
def client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def test_browser_personality_profile_defaults(client: TestClient) -> None:
    response = client.get("/browser/personality")
    assert response.status_code == 200
    data = response.json()
    assert data["profile"]["preset"] in {"supportive", "playful", "professional", "concise", "custom"}


def test_whatsapp_busy_mode_policy_multi_platform_defaults(client: TestClient) -> None:
    response = client.get("/browser/whatsapp/busy-mode")
    assert response.status_code == 200
    policy = response.json()
    assert "whatsapp" in policy["enabledPlatforms"]
    assert "messenger" in policy["enabledPlatforms"]
    assert "discord" in policy["enabledPlatforms"]


def test_browser_personality_preview(client: TestClient) -> None:
    response = client.post(
        "/browser/personality/preview",
        json={"sampleText": "Please tell them I am currently in a meeting."},
    )
    assert response.status_code == 200
    assert "preview" in response.json()
