from __future__ import annotations

import json
import os
import shutil
import sqlite3
import time
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .chat import ChatStore
from .connectors import ConnectorManager
from .identity import PRODUCT_NAME
from .local_models import ModelRouter
from .memory import MemoryStore
from .models import (
    ConnectorHealthItem,
    ConnectorHealthResponse,
    DeleteLocalDataResponse,
    PerformanceMetric,
    PerformanceProfileResponse,
    ReleaseLogFile,
    ReleaseLogListResponse,
    ReleaseLogReadResponse,
    ReleasePrivacyExportResponse,
    ReleaseReadinessItem,
    ReleaseReadinessResponse,
    ReleaseUpdatePlanResponse,
    CrashRecoveryResponse,
)
from .privacy import PrivacyFirewall
from .runtime_time import utc_timestamp
from .settings import CoreSettings
from .storage import CoreStore
from .voice import LocalVoiceService


DELETE_CONFIRMATION_PHRASE = "DELETE LOCAL DATA"


class ReleaseSafetyError(RuntimeError):
    pass


class ReleaseService:
    def __init__(
        self,
        *,
        settings: CoreSettings,
        store: CoreStore,
        memory_store: MemoryStore,
        chat_store: ChatStore,
        privacy_firewall: PrivacyFirewall,
        connector_manager: ConnectorManager,
        model_router: ModelRouter,
        voice_service: LocalVoiceService,
        repo_root: Path | None = None,
    ) -> None:
        self.settings = settings
        self.store = store
        self.memory_store = memory_store
        self.chat_store = chat_store
        self.privacy_firewall = privacy_firewall
        self.connector_manager = connector_manager
        self.model_router = model_router
        self.voice_service = voice_service
        self.repo_root = repo_root or Path(__file__).resolve().parents[4]
        self.session_id = f"release_{uuid.uuid4().hex}"
        self.state_path = settings.data_dir / "release-state.json"
        self.previous_state: dict[str, Any] = {}

    def mark_startup(self) -> None:
        self.previous_state = read_json(self.state_path)
        payload = {
            "schemaVersion": 1,
            "currentSessionId": self.session_id,
            "startedAt": utc_timestamp(),
            "cleanShutdown": False,
            "previous": self.previous_state,
        }
        write_json(self.state_path, payload)

    def mark_clean_shutdown(self) -> None:
        state = read_json(self.state_path)
        state.update(
            {
                "currentSessionId": self.session_id,
                "cleanShutdown": True,
                "cleanShutdownAt": utc_timestamp(),
            }
        )
        write_json(self.state_path, state)

    def crash_recovery(self) -> CrashRecoveryResponse:
        current = read_json(self.state_path)
        previous = current.get("previous") if isinstance(current.get("previous"), dict) else self.previous_state
        previous_crash = bool(previous) and previous.get("cleanShutdown") is False
        actions = [
            "Core process state is persisted at startup and clean shutdown.",
            "Desktop shell restarts the core process when the user requests recovery.",
        ]
        if previous_crash:
            actions.insert(0, "Previous core session did not record a clean shutdown.")

        return CrashRecoveryResponse(
            current_session_id=self.session_id,
            previous_crash_detected=previous_crash,
            started_at=str(current.get("startedAt") or utc_timestamp()),
            last_started_at=previous.get("startedAt") if isinstance(previous, dict) else None,
            last_clean_shutdown_at=previous.get("cleanShutdownAt") if isinstance(previous, dict) else None,
            recovery_actions=actions,
        )

    def readiness(self, version: str) -> ReleaseReadinessResponse:
        tauri_config = read_json(self.repo_root / "apps" / "desktop" / "src-tauri" / "tauri.conf.json")
        root_package = read_json(self.repo_root / "package.json")
        desktop_package = read_json(self.repo_root / "apps" / "desktop" / "package.json")
        bundle = tauri_config.get("bundle") if isinstance(tauri_config.get("bundle"), dict) else {}
        scripts = root_package.get("scripts") if isinstance(root_package.get("scripts"), dict) else {}
        desktop_scripts = desktop_package.get("scripts") if isinstance(desktop_package.get("scripts"), dict) else {}
        public_release_docs = [
            self.repo_root / "README.md",
            self.repo_root / "ARCHITECTURE.md",
            self.repo_root / "SECURITY.md",
        ]
        release_docs_ready = all(path.is_file() for path in public_release_docs)

        items = [
            check_item(
                "installer_bundle",
                "Installer bundle",
                bool(bundle.get("active")),
                "Tauri bundling is enabled." if bundle.get("active") else "Enable Tauri bundle.active for installer builds.",
            ),
            check_item(
                "installer_target",
                "Windows installer target",
                bool(bundle.get("targets")),
                f"Configured targets: {bundle.get('targets')}" if bundle.get("targets") else "No installer target configured.",
            ),
            check_item(
                "desktop_build_script",
                "Desktop build script",
                "desktop:build" in scripts and "build" in desktop_scripts,
                "Build scripts are present for frontend and Tauri.",
            ),
            check_item(
                "core_service",
                "Core service source",
                (self.repo_root / "services" / "core" / "src" / "deyana_core").is_dir(),
                "Core service source is available for bundled or development launch.",
            ),
            check_item(
                "privacy_firewall",
                "Privacy firewall",
                True,
                "Privacy firewall is initialized and audited locally.",
            ),
            check_item(
                "update_plan",
                "Public release guidance",
                release_docs_ready,
                "Public README, architecture, and security guidance are documented."
                if release_docs_ready
                else "Create README.md, ARCHITECTURE.md, and SECURITY.md.",
            ),
            check_item(
                "local_model_setup",
                "Local AI setup",
                bool(self.store.read_settings().selected_chat_model),
                f"Selected chat model: {self.store.read_settings().selected_chat_model}",
            ),
        ]
        blocked = any(item.status in {"missing", "blocked"} for item in items)
        update_ready = any(item.id == "update_plan" and item.status == "ready" for item in items)
        return ReleaseReadinessResponse(
            installer_ready=not blocked,
            update_plan_ready=update_ready,
            checked_at=utc_timestamp(),
            items=items,
        )

    def update_plan(self, version: str) -> ReleaseUpdatePlanResponse:
        return ReleaseUpdatePlanResponse(
            current_version=version,
            automatic_updates_enabled=False,
            plan=[
                "Build a signed installer from the Tauri bundle.",
                "Publish release notes with privacy-impact changes called out explicitly.",
                "Ship updates manually until a signed updater endpoint is introduced.",
                "Run backend tests, desktop TypeScript check, Tauri cargo check, and production UI build before release.",
                "Never enable automatic updates without a signed manifest and rollback plan.",
            ],
            checked_at=utc_timestamp(),
        )

    def list_logs(self) -> ReleaseLogListResponse:
        files = sorted(
            [log_file_item(path, self.log_relative_path(path)) for path in self.log_files()],
            key=lambda item: item.modified_at,
            reverse=True,
        )
        return ReleaseLogListResponse(files=files, total=len(files))

    def read_log(self, relative_path: str, max_characters: int = 20000) -> ReleaseLogReadResponse:
        path = self.resolve_log_path(relative_path)
        content = path.read_text(encoding="utf-8", errors="replace")
        truncated = len(content) > max_characters
        if truncated:
            content = content[-max_characters:]
        stat = path.stat()
        return ReleaseLogReadResponse(
            path=self.log_relative_path(path),
            content=content,
            truncated=truncated,
            size_bytes=stat.st_size,
            modified_at=timestamp_from_stat(stat.st_mtime),
        )

    def privacy_export(self) -> ReleasePrivacyExportResponse:
        memory = self.memory_store.export()
        chat_messages = self.chat_store.history(limit=200)
        privacy_audit = self.privacy_firewall.list_events(limit=200)
        connectors = self.connector_manager.list_connectors()
        sync_runs = self.connector_manager.list_sync_runs(limit=100)
        voice_settings = self.voice_service.read_settings()
        voice_status = self.voice_service.status()
        model_status = self.model_router.status()
        settings = self.store.read_settings()
        onboarding = self.store.read_onboarding()

        sections = {
            "settings": settings.model_dump(mode="json", by_alias=True),
            "onboarding": onboarding.model_dump(mode="json", by_alias=True),
            "memory": memory.model_dump(mode="json", by_alias=True),
            "chat": {"messages": [message.model_dump(mode="json", by_alias=True) for message in chat_messages]},
            "privacyAudit": privacy_audit.model_dump(mode="json", by_alias=True),
            "connectors": connectors.model_dump(mode="json", by_alias=True),
            "connectorSyncRuns": sync_runs.model_dump(mode="json", by_alias=True),
            "voice": {
                "settings": voice_settings.model_dump(mode="json", by_alias=True),
                "status": voice_status.model_dump(mode="json", by_alias=True),
            },
            "models": model_status.model_dump(mode="json", by_alias=True),
        }
        counts = {
            "memoryItems": len(memory.items),
            "chatMessages": len(chat_messages),
            "privacyAuditEvents": privacy_audit.total,
            "connectors": len(connectors.items),
            "connectorSyncRuns": sync_runs.total,
        }
        return ReleasePrivacyExportResponse(
            exported_at=utc_timestamp(),
            sections=sections,
            counts=counts,
            notes=[
                "Connector OAuth token secrets are not exported.",
                "Raw voice audio is not stored and is therefore not exported.",
                "Markdown vault files remain user-owned on disk.",
            ],
        )

    def connector_health(self) -> ConnectorHealthResponse:
        items = [health_for_connector(connector) for connector in self.connector_manager.list_connectors().items]
        healthy = sum(1 for item in items if item.health == "healthy")
        errors = sum(1 for item in items if item.health == "error")
        attention = sum(1 for item in items if item.health == "attention")
        return ConnectorHealthResponse(
            checked_at=utc_timestamp(),
            items=items,
            healthy=healthy,
            attention=attention,
            errors=errors,
        )

    def performance_profile(self, uptime_seconds: float) -> PerformanceProfileResponse:
        started = time.perf_counter()
        data_bytes = directory_size(self.settings.data_dir)
        log_bytes = sum(path.stat().st_size for path in self.log_files() if path.exists())
        db_bytes = sum(path.stat().st_size for path in self.settings.data_dir.glob("*.sqlite3") if path.exists())
        metrics = [
            PerformanceMetric(name="dataBytes", value=float(data_bytes), unit="bytes", detail="Core local data directory size."),
            PerformanceMetric(name="databaseBytes", value=float(db_bytes), unit="bytes", detail="SQLite database file size total."),
            PerformanceMetric(name="logBytes", value=float(log_bytes), unit="bytes", detail="Known release log file size total."),
            PerformanceMetric(name="memoryItems", value=float(sqlite_count(self.memory_store.database_path, "memory_items")), unit="count", detail="Structured memory items."),
            PerformanceMetric(name="chatMessages", value=float(sqlite_count(self.chat_store.database_path, "chat_messages")), unit="count", detail="Local chat messages."),
            PerformanceMetric(name="privacyAuditEvents", value=float(sqlite_count(self.privacy_firewall.database_path, "privacy_audit_events")), unit="count", detail="Privacy audit events."),
            PerformanceMetric(name="connectorSyncRuns", value=float(sqlite_count(self.connector_manager.database_path, "connector_sync_runs")), unit="count", detail="Connector sync run records."),
            PerformanceMetric(name="connectorItems", value=float(sqlite_count(self.connector_manager.database_path, "connector_items")), unit="count", detail="Normalized connector records."),
            PerformanceMetric(name="profileLatencyMs", value=round((time.perf_counter() - started) * 1000, 3), unit="ms", detail="Time to capture this lightweight profile."),
        ]
        return PerformanceProfileResponse(
            captured_at=utc_timestamp(),
            uptime_seconds=uptime_seconds,
            metrics=metrics,
        )

    def delete_local_data(self, *, confirmation_phrase: str, include_vault: bool) -> DeleteLocalDataResponse:
        if confirmation_phrase != DELETE_CONFIRMATION_PHRASE:
            raise ReleaseSafetyError(f'Type "{DELETE_CONFIRMATION_PHRASE}" to delete local data.')

        settings = self.store.read_settings()
        vault_path = Path(settings.vault_path).resolve(strict=False) if settings.vault_path else None
        deleted_paths: list[str] = []
        for path in self.delete_roots():
            deleted_paths.extend(delete_path_contents(path))

        vault_deleted = False
        if include_vault and vault_path:
            ensure_safe_delete_path(vault_path)
            if vault_path.exists():
                shutil.rmtree(vault_path)
                deleted_paths.append(str(vault_path))
                vault_deleted = True

        recreated = self.recreate_empty_stores()
        return DeleteLocalDataResponse(
            deleted=True,
            deleted_paths=deleted_paths,
            vault_deleted=vault_deleted,
            recreated_stores=recreated,
            restart_recommended=True,
            detail=(
                "Local app data was deleted and empty stores were recreated. "
                f"Restart {PRODUCT_NAME} before daily use."
            ),
        )

    def recreate_empty_stores(self) -> list[str]:
        self.store.reset_settings()
        self.store.write_onboarding(self.store.default_onboarding())
        self.memory_store.initialize()
        self.chat_store.initialize()
        self.privacy_firewall.initialize()
        self.connector_manager.initialize()
        self.voice_service.write_settings(self.voice_service.default_settings())
        self.mark_startup()
        return ["settings", "onboarding", "memory", "chat", "privacy", "connectors", "voice", "release-state"]

    def delete_roots(self) -> list[Path]:
        roots = [self.settings.data_dir, self.settings.log_dir]
        parent = self.settings.log_dir.parent
        if self.settings.log_dir.name == "core" and parent.name == "logs":
            roots.append(parent)
        unique: list[Path] = []
        for root in roots:
            resolved = root.resolve(strict=False)
            if resolved not in unique:
                ensure_safe_delete_path(resolved)
                unique.append(resolved)
        return unique

    def log_roots(self) -> list[Path]:
        roots = [self.settings.log_dir]
        parent = self.settings.log_dir.parent
        if self.settings.log_dir.name == "core" and parent.name == "logs":
            roots.append(parent)
        return unique_existing_dirs(roots)

    def log_files(self) -> list[Path]:
        files: list[Path] = []
        for root in self.log_roots():
            files.extend(path for path in root.rglob("*.log") if path.is_file())
        return sorted(set(files))

    def log_relative_path(self, path: Path) -> str:
        for root in self.log_roots():
            try:
                return path.resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                continue
        return path.name

    def resolve_log_path(self, relative_path: str) -> Path:
        normalized = relative_path.strip().replace("\\", "/")
        if not normalized or normalized.startswith("../") or "/../" in normalized:
            raise ReleaseSafetyError("Log path must stay inside the local log directory.")
        for root in self.log_roots():
            candidate = (root / normalized).resolve(strict=False)
            try:
                candidate.relative_to(root.resolve())
            except ValueError:
                continue
            if candidate.is_file():
                return candidate
        raise FileNotFoundError(relative_path)


def check_item(id: str, label: str, ok: bool, detail: str) -> ReleaseReadinessItem:
    return ReleaseReadinessItem(
        id=id,
        label=label,
        status="ready" if ok else "missing",
        detail=detail,
    )


def health_for_connector(connector) -> ConnectorHealthItem:
    if connector.status == "error" or connector.last_error:
        health = "error"
        detail = connector.last_error or "Connector is in an error state."
    elif connector.status == "syncing":
        health = "syncing"
        detail = "Connector sync is currently running."
    elif not connector.token_stored:
        health = "not_connected"
        detail = "Connector has no local token."
    elif not connector.enabled or connector.status == "paused":
        health = "paused"
        detail = "Connector is connected but sync is paused."
    elif not connector.last_sync_at:
        health = "attention"
        detail = "Connector is connected but has not synced yet."
    else:
        health = "healthy"
        detail = "Connector is connected and has a recorded sync."
    return ConnectorHealthItem(
        connector_id=connector.id,
        name=connector.name,
        health=health,
        status=connector.status,
        enabled=connector.enabled,
        token_stored=connector.token_stored,
        oauth_configured=connector.oauth_configured,
        last_sync_at=connector.last_sync_at,
        next_sync_at=connector.next_sync_at,
        last_error=connector.last_error,
        detail=detail,
    )


def log_file_item(path: Path, relative_path: str) -> ReleaseLogFile:
    stat = path.stat()
    return ReleaseLogFile(
        path=relative_path,
        name=path.name,
        size_bytes=stat.st_size,
        modified_at=timestamp_from_stat(stat.st_mtime),
    )


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = path.with_suffix(path.suffix + ".tmp")
    temp_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    temp_path.replace(path)


def timestamp_from_stat(value: float) -> str:
    return datetime.fromtimestamp(value, UTC).isoformat().replace("+00:00", "Z")


def unique_existing_dirs(paths: list[Path]) -> list[Path]:
    unique: list[Path] = []
    for path in paths:
        resolved = path.resolve(strict=False)
        if resolved.is_dir() and resolved not in unique:
            unique.append(resolved)
    return unique


def directory_size(path: Path) -> int:
    if not path.exists():
        return 0
    return sum(child.stat().st_size for child in path.rglob("*") if child.is_file())


def sqlite_count(database_path: Path, table: str) -> int:
    if not database_path.exists():
        return 0
    try:
        with sqlite3.connect(database_path) as connection:
            return int(connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0])
    except sqlite3.Error:
        return 0


def delete_path_contents(path: Path) -> list[str]:
    if not path.exists():
        return []
    ensure_safe_delete_path(path)
    deleted: list[str] = []
    for child in path.iterdir():
        try:
            if child.is_dir():
                shutil.rmtree(child)
            else:
                child.unlink()
            deleted.append(str(child))
        except PermissionError:
            if child.is_file() and clear_locked_file(child):
                deleted.append(f"{child} (cleared)")
            else:
                raise
    return deleted


def ensure_safe_delete_path(path: Path) -> None:
    resolved = path.resolve(strict=False)
    cwd = Path.cwd().resolve(strict=False)
    home = Path.home().resolve(strict=False)
    if resolved == cwd or resolved == home or resolved.parent == resolved:
        raise ReleaseSafetyError(f"Refusing to delete unsafe path: {resolved}")
    if len(resolved.parts) < 3:
        raise ReleaseSafetyError(f"Refusing to delete broad path: {resolved}")


def clear_locked_file(path: Path) -> bool:
    if path.suffix == ".sqlite3":
        try:
            with sqlite3.connect(path) as connection:
                rows = connection.execute(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
                ).fetchall()
                with connection:
                    for row in rows:
                        table = str(row[0]).replace('"', '""')
                        connection.execute(f'DELETE FROM "{table}"')
            return True
        except sqlite3.Error:
            return False

    try:
        path.write_text("", encoding="utf-8")
        return True
    except OSError:
        return False
