from __future__ import annotations

import json
import sqlite3
import uuid
from pathlib import Path

from .models import ChatMessageItem, ChatRole, MemorySourceReference, WebSourceReference
from .runtime_time import utc_timestamp


class ChatStore:
    def __init__(self, data_dir: Path) -> None:
        self.data_dir = data_dir
        self.database_path = data_dir / "chat.sqlite3"

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        with self.connect() as connection:
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS chat_messages (
                  id TEXT PRIMARY KEY,
                  role TEXT NOT NULL,
                  content TEXT NOT NULL,
                  model TEXT,
                  source_context_json TEXT NOT NULL DEFAULT '[]',
                  web_source_context_json TEXT NOT NULL DEFAULT '[]',
                  created_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at
                ON chat_messages(created_at);
                """
            )
            self.ensure_source_columns(connection)

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def append(
        self,
        role: ChatRole,
        content: str,
        model: str | None = None,
        source_references: list[MemorySourceReference] | None = None,
        web_source_references: list[WebSourceReference] | None = None,
    ) -> ChatMessageItem:
        self.initialize()
        references = source_references or []
        web_references = web_source_references or []
        message = ChatMessageItem(
            id=f"chat_{uuid.uuid4().hex}",
            role=role,
            content=content,
            model=model,
            source_references=references,
            web_source_references=web_references,
            created_at=utc_timestamp(),
        )
        with self.connect() as connection, connection:
            connection.execute(
                """
                INSERT INTO chat_messages (
                  id, role, content, model, source_context_json,
                  web_source_context_json, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    message.id,
                    message.role,
                    message.content,
                    message.model,
                    json.dumps(
                        [
                            reference.model_dump(mode="json", by_alias=True)
                            for reference in references
                        ]
                    ),
                    json.dumps(
                        [
                            reference.model_dump(mode="json", by_alias=True)
                            for reference in web_references
                        ]
                    ),
                    message.created_at,
                ),
            )
        return message

    def history(self, limit: int = 50) -> list[ChatMessageItem]:
        self.initialize()
        limit = max(1, min(limit, 200))
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM chat_messages
                ORDER BY created_at DESC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
        return [self.row_to_message(row) for row in reversed(rows)]

    def clear(self) -> int:
        self.initialize()
        with self.connect() as connection, connection:
            cursor = connection.execute("DELETE FROM chat_messages")
        return cursor.rowcount

    @staticmethod
    def row_to_message(row: sqlite3.Row) -> ChatMessageItem:
        try:
            source_references = [
                MemorySourceReference.model_validate(item)
                for item in json.loads(row["source_context_json"] or "[]")
                if isinstance(item, dict)
            ]
        except (json.JSONDecodeError, KeyError, TypeError):
            source_references = []

        try:
            web_source_references = [
                WebSourceReference.model_validate(item)
                for item in json.loads(row["web_source_context_json"] or "[]")
                if isinstance(item, dict)
            ]
        except (json.JSONDecodeError, KeyError, TypeError):
            web_source_references = []

        return ChatMessageItem(
            id=row["id"],
            role=row["role"],
            content=row["content"],
            model=row["model"],
            source_references=source_references,
            web_source_references=web_source_references,
            created_at=row["created_at"],
        )

    @staticmethod
    def ensure_source_columns(connection: sqlite3.Connection) -> None:
        columns = {
            row["name"]
            for row in connection.execute("PRAGMA table_info(chat_messages)").fetchall()
        }
        if "source_context_json" not in columns:
            connection.execute(
                "ALTER TABLE chat_messages ADD COLUMN source_context_json TEXT NOT NULL DEFAULT '[]'"
            )
        if "web_source_context_json" not in columns:
            connection.execute(
                "ALTER TABLE chat_messages ADD COLUMN web_source_context_json TEXT NOT NULL DEFAULT '[]'"
            )
