from __future__ import annotations

import hashlib
import json
import sqlite3
import uuid
from pathlib import Path

from ..runtime_time import utc_timestamp
from .models import (
    BrowserAuditDecision,
    BrowserAuditEvent,
    BrowserAuditListResponse,
    BrowserActionPlan,
    BrowserActionPlanListResponse,
    BrowserActionPlanStatus,
    BrowserActionStep,
    BrowserContactTonePreference,
    BrowserPageContext,
    BrowserPersonalityProfile,
    BrowserPermission,
    BrowserPermissionListResponse,
    BrowserSession,
    BrowserSessionListResponse,
    WhatsAppBusyModePolicy,
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

                CREATE TABLE IF NOT EXISTS browser_action_plans (
                  id TEXT PRIMARY KEY,
                  status TEXT NOT NULL,
                  summary TEXT NOT NULL,
                  preview_markdown TEXT NOT NULL,
                  origin TEXT,
                  page_session_id TEXT,
                  steps_json TEXT NOT NULL,
                  confirmation_token_sha256 TEXT,
                  expires_at TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  result_detail TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_browser_action_plans_updated_at
                ON browser_action_plans(updated_at);

                CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_policy (
                  id TEXT PRIMARY KEY,
                  enabled INTEGER NOT NULL,
                  allowlisted_contacts_json TEXT NOT NULL,
                  allow_groups INTEGER NOT NULL,
                  timezone TEXT NOT NULL,
                  window_start TEXT NOT NULL,
                  window_end TEXT NOT NULL,
                  cooldown_minutes INTEGER NOT NULL,
                  daily_limit INTEGER NOT NULL,
                  template TEXT NOT NULL,
                  emergency_stopped INTEGER NOT NULL,
                  permission_origin TEXT NOT NULL,
                  permission_granted INTEGER NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_events (
                  id TEXT PRIMARY KEY,
                  contact_label TEXT NOT NULL,
                  decision TEXT NOT NULL,
                  reason TEXT NOT NULL,
                  category TEXT NOT NULL,
                  urgent INTEGER NOT NULL,
                  created_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_browser_whatsapp_busy_events_contact_created_at
                ON browser_whatsapp_busy_events(contact_label, created_at);

                CREATE TABLE IF NOT EXISTS browser_personality_profile (
                  id TEXT PRIMARY KEY,
                  preset TEXT NOT NULL,
                  display_name TEXT NOT NULL,
                  custom_instruction TEXT NOT NULL,
                  writer_temperature REAL NOT NULL,
                  max_draft_characters INTEGER NOT NULL,
                  automation_disclosure TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS browser_contact_tone_preferences (
                  adapter_id TEXT NOT NULL,
                  contact_label TEXT NOT NULL,
                  tone_instruction TEXT NOT NULL,
                  approved INTEGER NOT NULL,
                  updated_at TEXT NOT NULL,
                  PRIMARY KEY (adapter_id, contact_label)
                );
                """
            )

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def upsert_context(self, context: BrowserPageContext, expires_at: str) -> BrowserSession:
        self.initialize()
        self._contexts[context.page_session_id] = context
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
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
        return BrowserSessionListResponse(items=[row_to_session(row) for row in rows], total=len(rows))

    def delete_session(self, page_session_id: str) -> bool:
        self._contexts.pop(page_session_id, None)
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute(
                    "DELETE FROM browser_sessions WHERE id = ?",
                    (page_session_id,),
                )
        return cursor.rowcount > 0

    def clear_sessions(self) -> int:
        self._contexts.clear()
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute("DELETE FROM browser_sessions")
        return cursor.rowcount

    def replace_permissions(self, permissions: list[BrowserPermission]) -> BrowserPermissionListResponse:
        self.initialize()
        with self.connect() as connection:
            with connection:
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
            payload_sha256=hashlib.sha256(payload.encode("utf-8")).hexdigest() if payload else None,
            payload_character_count=len(payload or ""),
            created_at=utc_timestamp(),
        )
        with self.connect() as connection:
            with connection:
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

    def create_action_plan(
        self,
        *,
        summary: str,
        preview_markdown: str,
        origin: str | None,
        page_session_id: str | None,
        steps: list[BrowserActionStep],
        confirmation_token: str,
        expires_at: str,
    ) -> BrowserActionPlan:
        self.initialize()
        timestamp = utc_timestamp()
        plan = BrowserActionPlan(
            id=f"browser_plan_{uuid.uuid4().hex}",
            status="pending_confirmation",
            summary=summary,
            preview_markdown=preview_markdown,
            origin=origin,
            page_session_id=page_session_id,
            steps=steps,
            confirmation_token=confirmation_token,
            expires_at=expires_at,
            created_at=timestamp,
            updated_at=timestamp,
        )
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO browser_action_plans (
                      id, status, summary, preview_markdown, origin, page_session_id,
                      steps_json, confirmation_token_sha256, expires_at, created_at,
                      updated_at, result_detail
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        plan.id,
                        plan.status,
                        plan.summary,
                        plan.preview_markdown,
                        plan.origin,
                        plan.page_session_id,
                        json.dumps([step.model_dump(mode="json", by_alias=True) for step in steps]),
                        token_digest(confirmation_token),
                        plan.expires_at,
                        plan.created_at,
                        plan.updated_at,
                        plan.result_detail,
                    ),
                )
        return plan

    def get_action_plan(self, plan_id: str) -> BrowserActionPlan:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM browser_action_plans WHERE id = ?",
                (plan_id,),
            ).fetchone()
        if not row:
            raise KeyError(plan_id)
        return row_to_action_plan(row)

    def list_action_plans(self, limit: int = 20) -> BrowserActionPlanListResponse:
        self.initialize()
        bounded_limit = max(1, min(limit, 100))
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM browser_action_plans ORDER BY updated_at DESC LIMIT ?",
                (bounded_limit,),
            ).fetchall()
            total = connection.execute(
                "SELECT COUNT(*) AS count FROM browser_action_plans"
            ).fetchone()["count"]
        return BrowserActionPlanListResponse(
            items=[row_to_action_plan(row) for row in rows],
            total=total,
        )

    def update_action_plan_status(
        self,
        plan_id: str,
        status: BrowserActionPlanStatus,
        *,
        result_detail: str | None = None,
        clear_token: bool = False,
    ) -> BrowserActionPlan:
        self.initialize()
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                if clear_token:
                    connection.execute(
                        """
                        UPDATE browser_action_plans
                        SET status = ?, updated_at = ?, result_detail = ?, confirmation_token_sha256 = NULL
                        WHERE id = ?
                        """,
                        (status, timestamp, result_detail, plan_id),
                    )
                else:
                    connection.execute(
                        """
                        UPDATE browser_action_plans
                        SET status = ?, updated_at = ?, result_detail = COALESCE(?, result_detail)
                        WHERE id = ?
                        """,
                        (status, timestamp, result_detail, plan_id),
                    )
        return self.get_action_plan(plan_id)

    def consume_action_plan_token(self, plan_id: str, confirmation_token: str) -> bool:
        self.initialize()
        digest = token_digest(confirmation_token)
        with self.connect() as connection:
            with connection:
                row = connection.execute(
                    """
                    SELECT confirmation_token_sha256 FROM browser_action_plans
                    WHERE id = ? AND status = 'pending_confirmation'
                    """,
                    (plan_id,),
                ).fetchone()
                if not row or not row["confirmation_token_sha256"]:
                    return False
                if row["confirmation_token_sha256"] != digest:
                    return False
                connection.execute(
                    """
                    UPDATE browser_action_plans
                    SET status = 'confirmed', confirmation_token_sha256 = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (utc_timestamp(), plan_id),
                )
        return True

    def cancel_pending_action_plans(self, reason: str) -> int:
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE browser_action_plans
                    SET status = 'cancelled', result_detail = ?, confirmation_token_sha256 = NULL, updated_at = ?
                    WHERE status IN ('pending_confirmation', 'confirmed', 'executing')
                    """,
                    (reason, utc_timestamp()),
                )
        return cursor.rowcount

    def get_whatsapp_busy_policy(self) -> WhatsAppBusyModePolicy:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM browser_whatsapp_busy_policy WHERE id = 'default'"
            ).fetchone()
        if not row:
            return default_whatsapp_busy_policy()
        return WhatsAppBusyModePolicy(
            enabled=bool(row["enabled"]),
            allowlisted_contacts=json.loads(row["allowlisted_contacts_json"]),
            allow_groups=bool(row["allow_groups"]),
            timezone=row["timezone"],
            window_start=row["window_start"],
            window_end=row["window_end"],
            cooldown_minutes=row["cooldown_minutes"],
            daily_limit=row["daily_limit"],
            template=row["template"],
            emergency_stopped=bool(row["emergency_stopped"]),
            permission_origin=row["permission_origin"],
            permission_granted=bool(row["permission_granted"]),
            updated_at=row["updated_at"],
        )

    def save_whatsapp_busy_policy(self, policy: WhatsAppBusyModePolicy) -> WhatsAppBusyModePolicy:
        self.initialize()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO browser_whatsapp_busy_policy (
                      id, enabled, allowlisted_contacts_json, allow_groups,
                      timezone, window_start, window_end, cooldown_minutes,
                      daily_limit, template, emergency_stopped, permission_origin,
                      permission_granted, updated_at
                    ) VALUES ('default', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                      enabled = excluded.enabled,
                      allowlisted_contacts_json = excluded.allowlisted_contacts_json,
                      allow_groups = excluded.allow_groups,
                      timezone = excluded.timezone,
                      window_start = excluded.window_start,
                      window_end = excluded.window_end,
                      cooldown_minutes = excluded.cooldown_minutes,
                      daily_limit = excluded.daily_limit,
                      template = excluded.template,
                      emergency_stopped = excluded.emergency_stopped,
                      permission_origin = excluded.permission_origin,
                      permission_granted = excluded.permission_granted,
                      updated_at = excluded.updated_at
                    """,
                    (
                        1 if policy.enabled else 0,
                        json.dumps(policy.allowlisted_contacts),
                        1 if policy.allow_groups else 0,
                        policy.timezone,
                        policy.window_start,
                        policy.window_end,
                        policy.cooldown_minutes,
                        policy.daily_limit,
                        policy.template,
                        1 if policy.emergency_stopped else 0,
                        policy.permission_origin,
                        1 if policy.permission_granted else 0,
                        policy.updated_at,
                    ),
                )
        return self.get_whatsapp_busy_policy()

    def set_whatsapp_busy_emergency_stop(self, active: bool) -> WhatsAppBusyModePolicy:
        policy = self.get_whatsapp_busy_policy().model_copy(
            update={"emergency_stopped": active, "updated_at": utc_timestamp()}
        )
        return self.save_whatsapp_busy_policy(policy)

    def record_whatsapp_busy_event(
        self,
        *,
        contact_label: str,
        decision: str,
        reason: str,
        category: str,
        urgent: bool,
    ) -> None:
        self.initialize()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO browser_whatsapp_busy_events (
                      id, contact_label, decision, reason, category, urgent, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        f"whatsapp_busy_{uuid.uuid4().hex}",
                        contact_label,
                        decision,
                        reason,
                        category,
                        1 if urgent else 0,
                        utc_timestamp(),
                    ),
                )

    def count_whatsapp_busy_events(
        self,
        *,
        contact_label: str | None = None,
        decision: str | None = None,
        since: str | None = None,
    ) -> int:
        self.initialize()
        clauses: list[str] = []
        params: list[str] = []
        if contact_label is not None:
            clauses.append("contact_label = ?")
            params.append(contact_label)
        if decision is not None:
            clauses.append("decision = ?")
            params.append(decision)
        if since is not None:
            clauses.append("created_at >= ?")
            params.append(since)
        where = f"WHERE {' AND '.join(clauses)}" if clauses else ""
        with self.connect() as connection:
            row = connection.execute(
                f"SELECT COUNT(*) AS count FROM browser_whatsapp_busy_events {where}",
                params,
            ).fetchone()
        return int(row["count"] if row else 0)

    def get_personality_profile(self) -> BrowserPersonalityProfile:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM browser_personality_profile WHERE id = 'default'"
            ).fetchone()
        if not row:
            return default_personality_profile()
        return BrowserPersonalityProfile.model_validate(dict(row))

    def save_personality_profile(self, profile: BrowserPersonalityProfile) -> BrowserPersonalityProfile:
        self.initialize()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO browser_personality_profile (
                      id, preset, display_name, custom_instruction,
                      writer_temperature, max_draft_characters,
                      automation_disclosure, updated_at
                    ) VALUES ('default', ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                      preset = excluded.preset,
                      display_name = excluded.display_name,
                      custom_instruction = excluded.custom_instruction,
                      writer_temperature = excluded.writer_temperature,
                      max_draft_characters = excluded.max_draft_characters,
                      automation_disclosure = excluded.automation_disclosure,
                      updated_at = excluded.updated_at
                    """,
                    (
                        profile.preset,
                        profile.display_name,
                        profile.custom_instruction,
                        profile.writer_temperature,
                        profile.max_draft_characters,
                        profile.automation_disclosure,
                        profile.updated_at,
                    ),
                )
        return self.get_personality_profile()

    def list_contact_tones(self) -> list[BrowserContactTonePreference]:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM browser_contact_tone_preferences ORDER BY adapter_id, contact_label"
            ).fetchall()
        return [BrowserContactTonePreference.model_validate(dict(row)) for row in rows]

    def get_contact_tone(self, adapter_id: str, contact_label: str) -> BrowserContactTonePreference | None:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                """
                SELECT * FROM browser_contact_tone_preferences
                WHERE adapter_id = ? AND contact_label = ?
                """,
                (adapter_id, contact_label),
            ).fetchone()
        return BrowserContactTonePreference.model_validate(dict(row)) if row else None

    def save_contact_tone(self, preference: BrowserContactTonePreference) -> BrowserContactTonePreference:
        self.initialize()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO browser_contact_tone_preferences (
                      adapter_id, contact_label, tone_instruction, approved, updated_at
                    ) VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT(adapter_id, contact_label) DO UPDATE SET
                      tone_instruction = excluded.tone_instruction,
                      approved = excluded.approved,
                      updated_at = excluded.updated_at
                    """,
                    (
                        preference.adapter_id,
                        preference.contact_label,
                        preference.tone_instruction,
                        1 if preference.approved else 0,
                        preference.updated_at,
                    ),
                )
        saved = self.get_contact_tone(preference.adapter_id, preference.contact_label)
        if saved is None:
            raise KeyError(preference.contact_label)
        return saved

    def delete_contact_tone(self, adapter_id: str, contact_label: str) -> bool:
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute(
                    """
                    DELETE FROM browser_contact_tone_preferences
                    WHERE adapter_id = ? AND contact_label = ?
                    """,
                    (adapter_id, contact_label),
                )
        return cursor.rowcount > 0


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


def row_to_action_plan(row: sqlite3.Row) -> BrowserActionPlan:
    steps_payload = json.loads(row["steps_json"])
    return BrowserActionPlan(
        id=row["id"],
        status=row["status"],
        summary=row["summary"],
        preview_markdown=row["preview_markdown"],
        origin=row["origin"],
        page_session_id=row["page_session_id"],
        steps=[BrowserActionStep.model_validate(item) for item in steps_payload],
        confirmation_token=None,
        expires_at=row["expires_at"],
        created_at=row["created_at"],
        updated_at=row["updated_at"],
        result_detail=row["result_detail"],
    )


def token_digest(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def default_whatsapp_busy_policy() -> WhatsAppBusyModePolicy:
    return WhatsAppBusyModePolicy(updated_at=utc_timestamp())


def default_personality_profile() -> BrowserPersonalityProfile:
    return BrowserPersonalityProfile(updated_at=utc_timestamp())
