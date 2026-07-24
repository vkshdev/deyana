from __future__ import annotations

import hashlib
import sqlite3
import uuid
from pathlib import Path

from ..runtime_time import utc_timestamp
from .models import (
    BrowserAuditDecision,
    BrowserAuditEvent,
    BrowserAuditListResponse,
    BrowserPageContext,
    BrowserPermission,
    BrowserPermissionListResponse,
    BrowserSession,
    BrowserSessionListResponse,
)


class BrowserStore:
    def __init__(self, data_dir: Path) -> None:
        self.database_path = data_dir / "browser.sqlite3"
        self._contexts: dict[str, BrowserPageContext] = {}

    def initialize(self) -> None:
        self.database_path.parent.mkdir(parents=True, exist_ok=True)
        with self.connect() as connection:
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS browser_sessions (
                  id TEXT PRIMARY KEY,
                  origin TEXT NOT NULL,
                  url TEXT NOT NULL,
                  title TEXT NOT NULL,
                  adapter_id TEXT NOT NULL,
                  mode TEXT NOT NULL,
                  character_count INTEGER NOT NULL,
                  truncated INTEGER NOT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  expires_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS browser_permissions (
                  origin TEXT NOT NULL,
                  kind TEXT NOT NULL,
                  granted INTEGER NOT NULL,
                  granted_at TEXT,
                  detail TEXT NOT NULL,
                  PRIMARY KEY (origin, kind)
                );

                CREATE TABLE IF NOT EXISTS browser_audit_events (
                  id TEXT PRIMARY KEY,
                  event_type TEXT NOT NULL,
                  decision TEXT NOT NULL,
                  operation TEXT NOT NULL,
                  origin TEXT,
                  page_session_id TEXT,
                  detail TEXT NOT NULL,
                  payload_sha256 TEXT,
                  payload_character_count INTEGER NOT NULL DEFAULT 0,
                  created_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_browser_audit_created_at
                ON browser_audit_events(created_at);
                """
            )

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def upsert_context(
        self, context: BrowserPageContext, expires_at: str
    ) -> BrowserSession:
        self.initialize()
        self._contexts[context.page_session_id] = context
        timestamp = utc_timestamp()
        with self.connect() as connection, connection:
            row = connection.execute(
                "SELECT created_at FROM browser_sessions WHERE id = ?",
                (context.page_session_id,),
            ).fetchone()
            connection.execute(
                """
                    INSERT INTO browser_sessions (
                      id, origin, url, title, adapter_id, mode,
                      character_count, truncated, created_at, updated_at, expires_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                      origin = excluded.origin,
                      url = excluded.url,
                      title = excluded.title,
                      adapter_id = excluded.adapter_id,
                      mode = excluded.mode,
                      character_count = excluded.character_count,
                      truncated = excluded.truncated,
                      updated_at = excluded.updated_at,
                      expires_at = excluded.expires_at
                    """,
                (
                    context.page_session_id,
                    context.origin,
                    context.url,
                    context.title,
                    context.adapter_id,
                    context.mode,
                    context.character_count,
                    1 if context.truncated else 0,
                    row["created_at"] if row else timestamp,
                    timestamp,
                    expires_at,
                ),
            )
        return self.get_session(context.page_session_id)

    def get_context(self, page_session_id: str) -> BrowserPageContext | None:
        return self._contexts.get(page_session_id)

    def get_session(self, page_session_id: str) -> BrowserSession:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM browser_sessions WHERE id = ?",
                (page_session_id,),
            ).fetchone()
        if not row:
            raise KeyError(page_session_id)
        return row_to_session(row)

    def list_sessions(self) -> BrowserSessionListResponse:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM browser_sessions ORDER BY updated_at DESC"
            ).fetchall()
        return BrowserSessionListResponse(
            items=[row_to_session(row) for row in rows], total=len(rows)
        )

    def delete_session(self, page_session_id: str) -> bool:
        self._contexts.pop(page_session_id, None)
        self.initialize()
        with self.connect() as connection, connection:
            cursor = connection.execute(
                "DELETE FROM browser_sessions WHERE id = ?",
                (page_session_id,),
            )
        return cursor.rowcount > 0

    def clear_sessions(self) -> int:
        self._contexts.clear()
        self.initialize()
        with self.connect() as connection, connection:
            cursor = connection.execute("DELETE FROM browser_sessions")
        return cursor.rowcount

    def replace_permissions(
        self, permissions: list[BrowserPermission]
    ) -> BrowserPermissionListResponse:
        self.initialize()
        with self.connect() as connection, connection:
            connection.execute("DELETE FROM browser_permissions")
            connection.executemany(
                """
                    INSERT INTO browser_permissions (
                      origin, kind, granted, granted_at, detail
                    ) VALUES (?, ?, ?, ?, ?)
                    """,
                [
                    (
                        item.origin,
                        item.kind,
                        1 if item.granted else 0,
                        item.granted_at,
                        item.detail,
                    )
                    for item in permissions
                ],
            )
        return self.list_permissions()

    def list_permissions(self) -> BrowserPermissionListResponse:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM browser_permissions ORDER BY origin, kind"
            ).fetchall()
        items = [
            BrowserPermission(
                origin=row["origin"],
                kind=row["kind"],
                granted=bool(row["granted"]),
                granted_at=row["granted_at"],
                detail=row["detail"],
            )
            for row in rows
        ]
        return BrowserPermissionListResponse(items=items, total=len(items))

    def record_audit(
        self,
        *,
        event_type: str,
        decision: BrowserAuditDecision,
        operation: str,
        detail: str,
        origin: str | None = None,
        page_session_id: str | None = None,
        payload: str | None = None,
    ) -> BrowserAuditEvent:
        self.initialize()
        event = BrowserAuditEvent(
            id=f"browser_audit_{uuid.uuid4().hex}",
            event_type=event_type,
            decision=decision,
            operation=operation,
            origin=origin,
            page_session_id=page_session_id,
            detail=detail,
            payload_sha256=hashlib.sha256(payload.encode("utf-8")).hexdigest()
            if payload
            else None,
            payload_character_count=len(payload or ""),
            created_at=utc_timestamp(),
        )
        with self.connect() as connection, connection:
            connection.execute(
                """
                    INSERT INTO browser_audit_events (
                      id, event_type, decision, operation, origin,
                      page_session_id, detail, payload_sha256,
                      payload_character_count, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                (
                    event.id,
                    event.event_type,
                    event.decision,
                    event.operation,
                    event.origin,
                    event.page_session_id,
                    event.detail,
                    event.payload_sha256,
                    event.payload_character_count,
                    event.created_at,
                ),
            )
        return event

    def list_audit(self, limit: int = 50) -> BrowserAuditListResponse:
        self.initialize()
        bounded_limit = max(1, min(limit, 200))
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM browser_audit_events ORDER BY created_at DESC LIMIT ?",
                (bounded_limit,),
            ).fetchall()
            total = connection.execute(
                "SELECT COUNT(*) AS count FROM browser_audit_events"
            ).fetchone()["count"]
        return BrowserAuditListResponse(
            items=[BrowserAuditEvent.model_validate(dict(row)) for row in rows],
            total=total,
        )


def row_to_session(row: sqlite3.Row) -> BrowserSession:
    return BrowserSession(
        id=row["id"],
        origin=row["origin"],
        url=row["url"],
        title=row["title"],
        adapter_id=row["adapter_id"],
        mode=row["mode"],
        character_count=row["character_count"],
        truncated=bool(row["truncated"]),
        created_at=row["created_at"],
        updated_at=row["updated_at"],
        expires_at=row["expires_at"],
    )
