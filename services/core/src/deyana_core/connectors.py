from __future__ import annotations

import json
import sqlite3
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from urllib.parse import urlencode

from .models import (
    ConnectorItem,
    ConnectorListResponse,
    ConnectorOAuthStartResponse,
    ConnectorSyncResponse,
    ConnectorSyncRun,
    ConnectorSyncRunsResponse,
    ConnectorSyncRunStatus,
    ConnectorStatus,
    PrivacyCheckRequest,
    PrivacyCheckResponse,
)
from .privacy import PrivacyFirewall, PrivacyPolicyError
from .runtime_time import utc_timestamp
from .token_vault import TokenVault


DEFAULT_SYNC_INTERVAL_MINUTES = 360


class ConnectorError(RuntimeError):
    pass


class ConnectorNotFoundError(ConnectorError):
    pass


class ConnectorStateError(ConnectorError):
    pass


@dataclass(frozen=True)
class ConnectorSyncResult:
    items_seen: int
    items_written: int
    detail: str


@dataclass(frozen=True)
class ConnectorDefinition:
    id: str
    name: str
    scopes: tuple[str, ...]
    authorization_url: str
    token_url: str
    api_probe_url: str
    default_sync_interval_minutes: int = DEFAULT_SYNC_INTERVAL_MINUTES


class BaseConnector:
    definition: ConnectorDefinition

    @property
    def id(self) -> str:
        return self.definition.id

    @property
    def name(self) -> str:
        return self.definition.name

    @property
    def scopes(self) -> list[str]:
        return list(self.definition.scopes)

    def build_authorization_url(self, *, state: str, redirect_uri: str) -> str:
        query = urlencode(
            {
                "client_id": "deyana-local-mock",
                "redirect_uri": redirect_uri,
                "response_type": "code",
                "scope": " ".join(self.definition.scopes),
                "state": state,
                "access_type": "offline",
                "prompt": "consent",
            }
        )
        return f"{self.definition.authorization_url}?{query}"

    def create_mock_token(self, *, code: str, issued_at: str) -> dict[str, object]:
        _ = code
        return {
            "schemaVersion": 1,
            "connectorId": self.id,
            "accessToken": f"mock_access_{uuid.uuid4().hex}",
            "refreshToken": f"mock_refresh_{uuid.uuid4().hex}",
            "tokenType": "Bearer",
            "scopes": self.scopes,
            "issuedAt": issued_at,
            "expiresAt": (datetime.now(UTC) + timedelta(hours=1)).isoformat().replace("+00:00", "Z"),
            "mock": True,
        }

    def sync(self, token: dict[str, object]) -> ConnectorSyncResult:
        if token.get("connectorId") != self.id:
            raise ConnectorStateError("Stored token belongs to a different connector.")
        return ConnectorSyncResult(items_seen=0, items_written=0, detail="Mock connector sync completed.")


class MockConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition


CONNECTOR_DEFINITIONS = [
    ConnectorDefinition(
        id="gmail",
        name="Gmail",
        scopes=("https://www.googleapis.com/auth/gmail.readonly",),
        authorization_url="https://accounts.google.com/o/oauth2/v2/auth",
        token_url="https://oauth2.googleapis.com/token",
        api_probe_url="https://gmail.googleapis.com/gmail/v1/users/me/messages",
    ),
    ConnectorDefinition(
        id="calendar",
        name="Calendar",
        scopes=("https://www.googleapis.com/auth/calendar.readonly",),
        authorization_url="https://accounts.google.com/o/oauth2/v2/auth",
        token_url="https://oauth2.googleapis.com/token",
        api_probe_url="https://calendar-json.googleapis.com/calendar/v3/users/me/calendarList",
    ),
    ConnectorDefinition(
        id="github",
        name="GitHub",
        scopes=("read:user", "repo"),
        authorization_url="https://github.com/login/oauth/authorize",
        token_url="https://github.com/login/oauth/access_token",
        api_probe_url="https://api.github.com/user/repos",
    ),
]


class ConnectorScheduler:
    def next_sync_at(self, connector: ConnectorItem, *, from_time: datetime | None = None) -> str | None:
        if not connector.enabled or not connector.token_stored or connector.status in {"not_connected", "error"}:
            return None
        base = from_time or datetime.now(UTC)
        return (base + timedelta(minutes=connector.sync_interval_minutes)).isoformat().replace("+00:00", "Z")

    def is_due(self, connector: ConnectorItem, *, at_time: datetime | None = None) -> bool:
        if not connector.next_sync_at:
            return False
        due_at = datetime.fromisoformat(connector.next_sync_at.replace("Z", "+00:00"))
        return due_at <= (at_time or datetime.now(UTC))


class ConnectorManager:
    def __init__(self, data_dir: Path, privacy_firewall: PrivacyFirewall) -> None:
        self.data_dir = data_dir
        self.database_path = data_dir / "connectors.sqlite3"
        self.privacy_firewall = privacy_firewall
        self.token_vault = TokenVault(data_dir, self.database_path)
        self.scheduler = ConnectorScheduler()
        self.registry = {definition.id: MockConnector(definition) for definition in CONNECTOR_DEFINITIONS}

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.token_vault.initialize()
        with self.connect() as connection:
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS connectors (
                  id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  status TEXT NOT NULL,
                  enabled INTEGER NOT NULL,
                  scopes_json TEXT NOT NULL,
                  sync_interval_minutes INTEGER NOT NULL,
                  last_sync_at TEXT,
                  next_sync_at TEXT,
                  token_stored INTEGER NOT NULL,
                  token_updated_at TEXT,
                  last_error TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS connector_oauth_states (
                  state TEXT PRIMARY KEY,
                  connector_id TEXT NOT NULL,
                  redirect_uri TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  expires_at TEXT NOT NULL,
                  used_at TEXT
                );

                CREATE TABLE IF NOT EXISTS connector_sync_runs (
                  id TEXT PRIMARY KEY,
                  connector_id TEXT NOT NULL,
                  status TEXT NOT NULL,
                  reason TEXT NOT NULL,
                  started_at TEXT NOT NULL,
                  completed_at TEXT,
                  items_seen INTEGER NOT NULL DEFAULT 0,
                  items_written INTEGER NOT NULL DEFAULT 0,
                  error_message TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_connector_sync_runs_started_at
                ON connector_sync_runs(started_at);
                """
            )

        for connector in self.registry.values():
            self.register(connector)

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def register(self, connector: BaseConnector) -> ConnectorItem:
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                existing = connection.execute(
                    "SELECT created_at FROM connectors WHERE id = ?",
                    (connector.id,),
                ).fetchone()
                connection.execute(
                    """
                    INSERT INTO connectors (
                      id, name, status, enabled, scopes_json, sync_interval_minutes,
                      last_sync_at, next_sync_at, token_stored, token_updated_at,
                      last_error, created_at, updated_at
                    )
                    VALUES (?, ?, 'not_connected', 0, ?, ?, NULL, NULL, 0, NULL, NULL, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                      name = excluded.name,
                      scopes_json = excluded.scopes_json,
                      updated_at = excluded.updated_at
                    """,
                    (
                        connector.id,
                        connector.name,
                        json.dumps(connector.scopes),
                        connector.definition.default_sync_interval_minutes,
                        existing["created_at"] if existing else timestamp,
                        timestamp,
                    ),
                )
        return self.get_connector(connector.id)

    def list_connectors(self) -> ConnectorListResponse:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute("SELECT * FROM connectors").fetchall()
        by_id = {row["id"]: self._row_to_connector(row) for row in rows}
        return ConnectorListResponse(items=[by_id[connector_id] for connector_id in self.registry if connector_id in by_id])

    def get_connector(self, connector_id: str) -> ConnectorItem:
        with self.connect() as connection:
            row = connection.execute("SELECT * FROM connectors WHERE id = ?", (connector_id,)).fetchone()
        if not row:
            raise ConnectorNotFoundError(f"Unknown connector: {connector_id}")
        return self._row_to_connector(row)

    def update_settings(
        self,
        connector_id: str,
        *,
        enabled: bool | None = None,
        sync_interval_minutes: int | None = None,
    ) -> ConnectorItem:
        connector = self.get_connector(connector_id)
        next_enabled = connector.enabled if enabled is None else enabled
        next_interval = connector.sync_interval_minutes if sync_interval_minutes is None else sync_interval_minutes
        status: ConnectorStatus = connector.status
        if connector.token_stored:
            status = "connected" if next_enabled else "paused"
        next_sync_at = self.scheduler.next_sync_at(
            connector.model_copy(
                update={
                    "enabled": next_enabled,
                    "sync_interval_minutes": next_interval,
                    "status": status,
                }
            )
        )
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET enabled = ?, sync_interval_minutes = ?, status = ?,
                        next_sync_at = ?, last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (
                        1 if next_enabled else 0,
                        next_interval,
                        status,
                        next_sync_at,
                        timestamp,
                        connector_id,
                    ),
                )
        return self.get_connector(connector_id)

    def start_oauth(self, connector_id: str, redirect_uri: str | None = None) -> ConnectorOAuthStartResponse:
        connector = self._registered_connector(connector_id)
        redirect = redirect_uri or "deyana://oauth/callback"
        state = f"oauth_{uuid.uuid4().hex}"
        created_at = utc_timestamp()
        expires_at = (datetime.now(UTC) + timedelta(minutes=10)).isoformat().replace("+00:00", "Z")
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO connector_oauth_states (
                      state, connector_id, redirect_uri, created_at, expires_at, used_at
                    )
                    VALUES (?, ?, ?, ?, ?, NULL)
                    """,
                    (state, connector_id, redirect, created_at, expires_at),
                )
        return ConnectorOAuthStartResponse(
            connector=self.get_connector(connector_id),
            authorization_url=connector.build_authorization_url(state=state, redirect_uri=redirect),
            state=state,
            scopes=connector.scopes,
            redirect_uri=redirect,
            expires_at=expires_at,
            mock=True,
        )

    def complete_oauth(
        self,
        connector_id: str,
        *,
        state: str,
        code: str,
        user_approved: bool,
    ) -> tuple[ConnectorItem, PrivacyCheckResponse]:
        connector = self._registered_connector(connector_id)
        oauth_state = self._consume_oauth_state(connector_id, state)
        privacy_result = self.privacy_firewall.check(
            PrivacyCheckRequest(
                url=connector.definition.token_url,
                method="POST",
                purpose="oauth_api_fetch",
                data_category="oauth_token",
                payload_preview=f"{connector.name} OAuth token exchange",
                user_approved=user_approved,
                connector_id=connector_id,
            )
        )
        if not privacy_result.allowed:
            raise PrivacyPolicyError(privacy_result)

        issued_at = utc_timestamp()
        token_payload = connector.create_mock_token(code=code, issued_at=issued_at)
        token_updated_at = self.token_vault.store(connector_id, token_payload)
        current = self.get_connector(connector_id)
        next_sync_at = self.scheduler.next_sync_at(
            current.model_copy(update={"enabled": True, "token_stored": True, "status": "connected"})
        )
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connector_oauth_states
                    SET used_at = ?
                    WHERE state = ?
                    """,
                    (issued_at, oauth_state["state"]),
                )
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'connected', enabled = 1, token_stored = 1,
                        token_updated_at = ?, next_sync_at = ?, last_error = NULL,
                        updated_at = ?
                    WHERE id = ?
                    """,
                    (token_updated_at, next_sync_at, issued_at, connector_id),
                )
        return self.get_connector(connector_id), privacy_result

    def disconnect(self, connector_id: str) -> tuple[ConnectorItem, bool]:
        self._registered_connector(connector_id)
        token_deleted = self.token_vault.delete(connector_id)
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'not_connected', enabled = 0, token_stored = 0,
                        token_updated_at = NULL, last_sync_at = NULL, next_sync_at = NULL,
                        last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (timestamp, connector_id),
                )
        return self.get_connector(connector_id), token_deleted

    def start_sync(self, connector_id: str, *, reason: str = "manual") -> ConnectorSyncResponse:
        connector = self.get_connector(connector_id)
        if not connector.token_stored:
            run = self._create_sync_run(connector_id, reason=reason, status="failed")
            run = self._complete_sync_run(run.id, status="failed", error_message="Connector is not connected.")
            self._set_connector_error(connector_id, "Connector is not connected.")
            raise ConnectorStateError("Connector is not connected.")
        if not connector.enabled:
            run = self._create_sync_run(connector_id, reason=reason, status="skipped")
            run = self._complete_sync_run(run.id, status="skipped", error_message="Connector sync is paused.")
            return ConnectorSyncResponse(connector=connector, run=run)

        run = self._create_sync_run(connector_id, reason=reason, status="running")
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'syncing', last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (timestamp, connector_id),
                )
        return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run)

    def finish_sync(self, connector_id: str, run_id: str) -> tuple[ConnectorSyncResponse, PrivacyCheckResponse | None]:
        connector = self._registered_connector(connector_id)
        token = self.token_vault.read(connector_id)
        if not token:
            run = self._complete_sync_run(run_id, status="failed", error_message="Connector token is missing.")
            self._set_connector_error(connector_id, "Connector token is missing.")
            raise ConnectorStateError("Connector token is missing.")

        privacy_result = self.privacy_firewall.check(
            PrivacyCheckRequest(
                url=connector.definition.api_probe_url,
                method="GET",
                purpose="connector_api_fetch",
                data_category="connector_metadata",
                payload_preview=f"{connector.name} metadata sync",
                user_approved=True,
                connector_id=connector_id,
            )
        )
        if not privacy_result.allowed:
            run = self._complete_sync_run(run_id, status="failed", error_message=privacy_result.reason)
            self._set_connector_error(connector_id, privacy_result.reason)
            raise PrivacyPolicyError(privacy_result)

        try:
            result = connector.sync(token)
        except ConnectorError as error:
            run = self._complete_sync_run(run_id, status="failed", error_message=str(error))
            self._set_connector_error(connector_id, str(error))
            return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run), privacy_result

        run = self._complete_sync_run(
            run_id,
            status="completed",
            items_seen=result.items_seen,
            items_written=result.items_written,
        )
        current = self.get_connector(connector_id)
        completed_at = run.completed_at or utc_timestamp()
        next_sync_at = self.scheduler.next_sync_at(current.model_copy(update={"status": "connected"}))
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'connected', last_sync_at = ?, next_sync_at = ?,
                        last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (completed_at, next_sync_at, completed_at, connector_id),
                )
        return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run), privacy_result

    def list_sync_runs(self, *, limit: int = 20) -> ConnectorSyncRunsResponse:
        self.initialize()
        limit = max(1, min(limit, 100))
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM connector_sync_runs
                ORDER BY started_at DESC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
            total = connection.execute("SELECT COUNT(*) AS count FROM connector_sync_runs").fetchone()["count"]
        return ConnectorSyncRunsResponse(items=[self._row_to_run(row) for row in rows], total=total)

    def due_connectors(self) -> list[ConnectorItem]:
        return [connector for connector in self.list_connectors().items if self.scheduler.is_due(connector)]

    def _registered_connector(self, connector_id: str) -> BaseConnector:
        connector = self.registry.get(connector_id)
        if not connector:
            raise ConnectorNotFoundError(f"Unknown connector: {connector_id}")
        return connector

    def _consume_oauth_state(self, connector_id: str, state: str) -> sqlite3.Row:
        with self.connect() as connection:
            row = connection.execute(
                """
                SELECT * FROM connector_oauth_states
                WHERE state = ? AND connector_id = ?
                """,
                (state, connector_id),
            ).fetchone()
        if not row:
            raise ConnectorStateError("OAuth state is unknown.")
        if row["used_at"]:
            raise ConnectorStateError("OAuth state has already been used.")
        expires_at = datetime.fromisoformat(row["expires_at"].replace("Z", "+00:00"))
        if expires_at <= datetime.now(UTC):
            raise ConnectorStateError("OAuth state has expired.")
        return row

    def _create_sync_run(
        self,
        connector_id: str,
        *,
        reason: str,
        status: ConnectorSyncRunStatus,
    ) -> ConnectorSyncRun:
        run_id = f"sync_{uuid.uuid4().hex}"
        started_at = utc_timestamp()
        completed_at = started_at if status in {"completed", "failed", "skipped"} else None
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO connector_sync_runs (
                      id, connector_id, status, reason, started_at, completed_at,
                      items_seen, items_written, error_message
                    )
                    VALUES (?, ?, ?, ?, ?, ?, 0, 0, NULL)
                    """,
                    (run_id, connector_id, status, reason, started_at, completed_at),
                )
        return self._get_sync_run(run_id)

    def _complete_sync_run(
        self,
        run_id: str,
        *,
        status: ConnectorSyncRunStatus,
        items_seen: int = 0,
        items_written: int = 0,
        error_message: str | None = None,
    ) -> ConnectorSyncRun:
        completed_at = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connector_sync_runs
                    SET status = ?, completed_at = ?, items_seen = ?,
                        items_written = ?, error_message = ?
                    WHERE id = ?
                    """,
                    (status, completed_at, items_seen, items_written, error_message, run_id),
                )
        return self._get_sync_run(run_id)

    def _get_sync_run(self, run_id: str) -> ConnectorSyncRun:
        with self.connect() as connection:
            row = connection.execute("SELECT * FROM connector_sync_runs WHERE id = ?", (run_id,)).fetchone()
        if not row:
            raise ConnectorStateError("Sync run is missing.")
        return self._row_to_run(row)

    def _set_connector_error(self, connector_id: str, message: str) -> None:
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'error', last_error = ?, updated_at = ?
                    WHERE id = ?
                    """,
                    (message, timestamp, connector_id),
                )

    def _row_to_connector(self, row: sqlite3.Row) -> ConnectorItem:
        return ConnectorItem(
            id=row["id"],
            name=row["name"],
            status=row["status"],
            enabled=bool(row["enabled"]),
            scopes=json.loads(row["scopes_json"]),
            sync_interval_minutes=row["sync_interval_minutes"],
            last_sync_at=row["last_sync_at"],
            next_sync_at=row["next_sync_at"],
            token_stored=bool(row["token_stored"]),
            token_updated_at=row["token_updated_at"],
            last_error=row["last_error"],
            updated_at=row["updated_at"],
        )

    def _row_to_run(self, row: sqlite3.Row) -> ConnectorSyncRun:
        return ConnectorSyncRun(
            id=row["id"],
            connector_id=row["connector_id"],
            status=row["status"],
            reason=row["reason"],
            started_at=row["started_at"],
            completed_at=row["completed_at"],
            items_seen=row["items_seen"],
            items_written=row["items_written"],
            error_message=row["error_message"],
        )
