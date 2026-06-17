from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings
from deyana_core.storage import VAULT_FOLDERS


def make_client(tmp_path):
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def test_onboarding_state_defaults_to_incomplete(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.get("/onboarding/state")

    assert response.status_code == 200
    body = response.json()
    assert body["completed"] is False
    assert body["currentStep"] == "welcome"
    assert body["selectedPrivacyMode"] == "local_only"
    assert body["selectedModelProfile"] == "low_spec"


def test_vault_select_creates_template_folders(tmp_path) -> None:
    vault_path = tmp_path / "DeyanaVault"

    with make_client(tmp_path) as client:
        response = client.post("/vault/select", json={"path": str(vault_path)})

    assert response.status_code == 200
    body = response.json()
    assert body["state"]["vaultStatus"] == "ready"
    assert body["settings"]["vaultPath"] == str(vault_path)
    assert sorted(body["createdFolders"]) == sorted(VAULT_FOLDERS)
    assert (vault_path / ".deyana-vault.json").is_file()
    assert all((vault_path / folder).is_dir() for folder in VAULT_FOLDERS)


def test_onboarding_completion_persists_across_core_restart(tmp_path) -> None:
    vault_path = tmp_path / "DeyanaVault"

    with make_client(tmp_path) as client:
        response = client.post(
            "/onboarding/complete",
            json={
                "privacyMode": "local_only",
                "modelProfile": "low_spec",
                "vaultPath": str(vault_path),
            },
        )

    assert response.status_code == 200
    assert response.json()["state"]["completed"] is True

    with make_client(tmp_path) as reopened_client:
        state_response = reopened_client.get("/onboarding/state")
        settings_response = reopened_client.get("/settings")

    state = state_response.json()
    settings = settings_response.json()
    assert state["completed"] is True
    assert state["currentStep"] == "complete"
    assert state["selectedVaultPath"] == str(vault_path)
    assert settings["onboardingCompleted"] is True
    assert settings["vaultPath"] == str(vault_path)


def test_completion_requires_vault_selection(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.post(
            "/onboarding/complete",
            json={"privacyMode": "local_only", "modelProfile": "low_spec"},
        )

    assert response.status_code == 400
    assert "vault" in response.json()["detail"].lower()
