from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .identity import PRODUCT_BRAND, PRODUCT_NAME
from .models import (
    AppSettings,
    ModelProfile,
    OnboardingState,
    PrivacyMode,
    SettingsPatch,
    SyncMode,
)
from .runtime_time import utc_timestamp

VAULT_FOLDERS = [
    "Daily",
    "Projects",
    "People",
    "Meetings",
    "Emails",
    "GitHub",
    "Slack",
    "Tasks",
    "Decisions",
    "Sources",
    "Inbox",
]


class CoreStore:
    def __init__(self, data_dir: Path) -> None:
        self.data_dir = data_dir
        self.settings_path = data_dir / "app-settings.json"
        self.onboarding_path = data_dir / "onboarding-state.json"

    def read_settings(self) -> AppSettings:
        data = self._read_json(self.settings_path)
        if not data:
            return self.default_settings()

        return AppSettings.model_validate({**self.default_settings().model_dump(), **data})

    def patch_settings(self, patch: SettingsPatch) -> AppSettings:
        settings = self.read_settings()
        updates = patch.model_dump(exclude_unset=True)
        next_settings = settings.model_copy(update={**updates, "updated_at": utc_timestamp()})
        self.write_settings(next_settings)
        return next_settings

    def write_settings(self, settings: AppSettings) -> None:
        self._write_json(self.settings_path, settings.model_dump(mode="json", by_alias=True))

    def reset_settings(self) -> AppSettings:
        settings = self.default_settings()
        self.write_settings(settings)
        return settings

    def read_onboarding(self) -> OnboardingState:
        data = self._read_json(self.onboarding_path)
        if not data:
            return self.default_onboarding()

        state = OnboardingState.model_validate({**self.default_onboarding().model_dump(), **data})
        if state.selected_vault_path and state.vault_status == "ready":
            vault_path = Path(state.selected_vault_path)
            if not vault_path.exists():
                state = state.model_copy(update={"vault_status": "missing"})
        return state

    def write_onboarding(self, state: OnboardingState) -> None:
        self._write_json(self.onboarding_path, state.model_dump(mode="json", by_alias=True))

    def select_vault(self, raw_path: str) -> tuple[OnboardingState, AppSettings, list[str]]:
        vault_path = self._normalize_vault_path(raw_path)
        created_folders = create_vault_template(vault_path)

        settings = self.read_settings().model_copy(
            update={"vault_path": str(vault_path), "updated_at": utc_timestamp()}
        )
        self.write_settings(settings)

        state = self.read_onboarding().model_copy(
            update={
                "current_step": "vault",
                "selected_vault_path": str(vault_path),
                "vault_status": "ready",
                "vault_error": None,
                "vault_folders": VAULT_FOLDERS,
            }
        )
        self.write_onboarding(state)
        return state, settings, created_folders

    def complete_onboarding(
        self,
        privacy_mode: PrivacyMode,
        model_profile: ModelProfile,
        vault_path: str | None = None,
    ) -> tuple[OnboardingState, AppSettings]:
        if vault_path:
            state, settings, _created = self.select_vault(vault_path)
        else:
            state = self.read_onboarding()
            settings = self.read_settings()

        if not state.selected_vault_path:
            raise ValueError("Select a vault folder before completing onboarding.")

        vault_root = Path(state.selected_vault_path)
        if not vault_root.exists():
            raise ValueError("Selected vault folder is missing.")

        missing = [folder for folder in VAULT_FOLDERS if not (vault_root / folder).is_dir()]
        if missing:
            create_vault_template(vault_root)

        timestamp = utc_timestamp()
        settings = settings.model_copy(
            update={
                "privacy_mode": privacy_mode,
                "model_profile": model_profile,
                "vault_path": str(vault_root),
                "onboarding_completed": True,
                "updated_at": timestamp,
            }
        )
        self.write_settings(settings)

        state = state.model_copy(
            update={
                "completed": True,
                "completed_at": timestamp,
                "current_step": "complete",
                "selected_privacy_mode": privacy_mode,
                "selected_model_profile": model_profile,
                "selected_vault_path": str(vault_root),
                "vault_status": "ready",
                "vault_error": None,
                "vault_folders": VAULT_FOLDERS,
            }
        )
        self.write_onboarding(state)
        return state, settings

    @staticmethod
    def default_settings() -> AppSettings:
        return AppSettings(updated_at=utc_timestamp())

    @staticmethod
    def default_onboarding() -> OnboardingState:
        return OnboardingState(vault_folders=VAULT_FOLDERS)

    def _read_json(self, path: Path) -> dict[str, Any]:
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def _write_json(self, path: Path, content: dict[str, Any]) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        temp_path = path.with_suffix(path.suffix + ".tmp")
        temp_path.write_text(json.dumps(content, indent=2), encoding="utf-8")
        temp_path.replace(path)

    @staticmethod
    def _normalize_vault_path(raw_path: str) -> Path:
        path = Path(raw_path.strip()).expanduser()
        if not str(path):
            raise ValueError("Vault path is required.")
        return path.resolve(strict=False)


def create_vault_template(vault_path: Path) -> list[str]:
    vault_path.mkdir(parents=True, exist_ok=True)
    created_folders: list[str] = []

    root_readme = vault_path / "README.md"
    if not root_readme.exists():
        root_readme.write_text(
            f"# {PRODUCT_BRAND} Vault\n\n"
            f"This vault is user-owned. {PRODUCT_NAME} writes compressed local summaries here, "
            "not raw private dumps by default.\n",
            encoding="utf-8",
        )

    manifest = vault_path / ".deyana-vault.json"
    manifest.write_text(
        json.dumps(
            {
                "schemaVersion": 1,
                "createdBy": "deyana-core",
                "folders": VAULT_FOLDERS,
            },
            indent=2,
        ),
        encoding="utf-8",
    )

    for folder in VAULT_FOLDERS:
        folder_path = vault_path / folder
        folder_path.mkdir(parents=True, exist_ok=True)
        created_folders.append(folder)

    return created_folders
