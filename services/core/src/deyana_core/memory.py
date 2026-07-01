from __future__ import annotations

import json
import re
import sqlite3
import uuid
from pathlib import Path

from .models import (
    DailySummaryRequest,
    MemoryCreateRequest,
    MemoryEntity,
    MemoryEntityListResponse,
    MemoryExportResponse,
    MemoryInsight,
    MemoryInsightType,
    MemoryInsightListResponse,
    MemoryItem,
    MemoryListResponse,
    MemoryReindexResponse,
    MemoryUpdateRequest,
    ProjectSummaryRequest,
)
from .memory_pipeline import analyze_memory, build_daily_summary, build_project_summary
from .runtime_time import utc_timestamp
from .storage import CoreStore, create_vault_template

TYPE_TO_FOLDER = {
    "daily_summary": "Daily",
    "project_summary": "Projects",
    "decision": "Decisions",
    "action_item": "Tasks",
    "git_summary": "GitHub",
    "connector_summary": "Inbox",
    "file_summary": "Sources",
    "chat": "Inbox",
    "note": "Inbox",
}

CONNECTOR_SOURCE_TO_FOLDER = {
    "gmail": "Emails",
    "calendar": "Meetings",
    "github": "GitHub",
    "drive": "Sources",
    "slack": "Slack",
    "notion": "Sources",
    "jira": "Tasks",
    "linear": "Tasks",
}


class MemoryStore:
    def __init__(self, data_dir: Path, core_store: CoreStore) -> None:
        self.data_dir = data_dir
        self.core_store = core_store
        self.database_path = data_dir / "memory.sqlite3"

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        with self.connect() as connection:
            connection.executescript(
                """
                PRAGMA foreign_keys = ON;

                CREATE TABLE IF NOT EXISTS memory_items (
                  id TEXT PRIMARY KEY,
                  type TEXT NOT NULL,
                  title TEXT NOT NULL,
                  summary TEXT NOT NULL,
                  content_markdown TEXT NOT NULL,
                  markdown_path TEXT,
                  source_type TEXT NOT NULL,
                  source_id TEXT,
                  source_uri TEXT,
                  importance INTEGER NOT NULL DEFAULT 3,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  deleted_at TEXT
                );

                CREATE TABLE IF NOT EXISTS memory_tags (
                  memory_id TEXT NOT NULL,
                  tag TEXT NOT NULL,
                  PRIMARY KEY (memory_id, tag),
                  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS memory_chunks (
                  id TEXT PRIMARY KEY,
                  memory_id TEXT NOT NULL,
                  chunk_text TEXT NOT NULL,
                  chunk_index INTEGER NOT NULL,
                  token_estimate INTEGER NOT NULL,
                  embedding_model TEXT,
                  embedding_ref TEXT,
                  created_at TEXT NOT NULL,
                  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS memory_entities (
                  id TEXT PRIMARY KEY,
                  memory_id TEXT NOT NULL,
                  name TEXT NOT NULL,
                  entity_type TEXT NOT NULL,
                  source_text TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS memory_insights (
                  id TEXT PRIMARY KEY,
                  memory_id TEXT NOT NULL,
                  type TEXT NOT NULL,
                  title TEXT NOT NULL,
                  detail TEXT NOT NULL,
                  status TEXT NOT NULL DEFAULT 'open',
                  due_at TEXT,
                  created_at TEXT NOT NULL,
                  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_memory_items_deleted_at ON memory_items(deleted_at);
                CREATE INDEX IF NOT EXISTS idx_memory_items_updated_at ON memory_items(updated_at);
                CREATE INDEX IF NOT EXISTS idx_memory_items_type ON memory_items(type);
                CREATE INDEX IF NOT EXISTS idx_memory_entities_name ON memory_entities(name);
                CREATE INDEX IF NOT EXISTS idx_memory_insights_type ON memory_insights(type);
                """
            )

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        connection.execute("PRAGMA foreign_keys = ON;")
        return connection

    def create(self, request: MemoryCreateRequest) -> MemoryItem:
        self.initialize()
        vault_root = self.require_vault_root()
        memory_id = f"memory_{uuid.uuid4().hex}"
        timestamp = utc_timestamp()
        analysis = analyze_memory(
            title=request.title,
            summary=request.summary,
            content_markdown=request.content_markdown,
            source_type=request.source_type,
            memory_type=request.type,
            existing_tags=request.tags,
            existing_importance=request.importance,
        )
        content_markdown = analysis.content_markdown
        folder = folder_for_memory(request.type, request.source_type)
        markdown_path = self.markdown_path(vault_root, folder, request.title, memory_id)
        markdown_content = self.render_markdown(
            memory_id=memory_id,
            memory_type=request.type,
            title=request.title,
            summary=analysis.summary,
            content_markdown=content_markdown,
            source_type=request.source_type,
            source_id=request.source_id,
            source_uri=request.source_uri,
            importance=analysis.importance,
            tags=list(analysis.tags),
            created_at=timestamp,
            updated_at=timestamp,
        )
        markdown_path.parent.mkdir(parents=True, exist_ok=True)
        markdown_path.write_text(markdown_content, encoding="utf-8")

        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO memory_items (
                      id, type, title, summary, content_markdown, markdown_path,
                      source_type, source_id, source_uri, importance,
                      created_at, updated_at, deleted_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
                    """,
                    (
                        memory_id,
                        request.type,
                        request.title,
                        analysis.summary,
                        content_markdown,
                        str(markdown_path),
                        request.source_type,
                        request.source_id,
                        request.source_uri,
                        analysis.importance,
                        timestamp,
                        timestamp,
                    ),
                )
                self.replace_tags(connection, memory_id, list(analysis.tags))
                self.replace_chunks(connection, memory_id, content_markdown, timestamp)
                self.replace_entities(connection, memory_id, analysis.entities, timestamp)
                self.replace_insights(
                    connection,
                    memory_id,
                    [*analysis.action_items, *analysis.decisions],
                    timestamp,
                )

        return self.get(memory_id)

    def list(self, query: str | None = None, limit: int = 20) -> MemoryListResponse:
        self.initialize()
        limit = max(1, min(limit, 100))
        query_value = (query or "").strip()
        params: list[object] = []
        where = "deleted_at IS NULL"

        if query_value:
            where += """
              AND (
                lower(title) LIKE ?
                OR lower(summary) LIKE ?
                OR lower(content_markdown) LIKE ?
                OR id IN (
                  SELECT memory_id FROM memory_tags WHERE lower(tag) LIKE ?
                )
                OR id IN (
                  SELECT memory_id FROM memory_entities WHERE lower(name) LIKE ?
                )
                OR id IN (
                  SELECT memory_id FROM memory_insights
                  WHERE lower(title) LIKE ? OR lower(detail) LIKE ?
                )
              )
            """
            pattern = f"%{query_value.lower()}%"
            params.extend([pattern, pattern, pattern, pattern, pattern, pattern, pattern])

        with self.connect() as connection:
            rows = connection.execute(
                f"""
                SELECT * FROM memory_items
                WHERE {where}
                ORDER BY updated_at DESC
                LIMIT ?
                """,
                [*params, limit],
            ).fetchall()
            total = connection.execute(
                f"SELECT COUNT(*) AS count FROM memory_items WHERE {where}",
                params,
            ).fetchone()["count"]

        return MemoryListResponse(
            items=[self.row_to_item(row) for row in rows],
            total=total,
            query=query_value or None,
        )

    def get(self, memory_id: str) -> MemoryItem:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM memory_items WHERE id = ? AND deleted_at IS NULL",
                (memory_id,),
            ).fetchone()

        if row is None:
            raise KeyError(memory_id)

        return self.row_to_item(row, read_markdown=True)

    def update(self, memory_id: str, request: MemoryUpdateRequest) -> MemoryItem:
        current = self.get(memory_id)
        timestamp = utc_timestamp()
        title = request.title if request.title is not None else current.title
        summary = request.summary if request.summary is not None else current.summary
        content_markdown = (
            request.content_markdown if request.content_markdown is not None else current.content_markdown
        )
        importance = request.importance if request.importance is not None else current.importance
        tags = request.tags if request.tags is not None else current.tags
        analysis = analyze_memory(
            title=title,
            summary=summary,
            content_markdown=content_markdown,
            source_type=current.source_type,
            memory_type=current.type,
            existing_tags=tags,
            existing_importance=importance,
        )
        summary = analysis.summary
        content_markdown = analysis.content_markdown
        importance = analysis.importance
        tags = list(analysis.tags)
        markdown_path = Path(current.markdown_path) if current.markdown_path else None

        if markdown_path:
            markdown_content = self.render_markdown(
                memory_id=memory_id,
                memory_type=current.type,
                title=title,
                summary=summary,
                content_markdown=content_markdown,
                source_type=current.source_type,
                source_id=current.source_id,
                source_uri=current.source_uri,
                importance=importance,
                tags=tags,
                created_at=current.created_at,
                updated_at=timestamp,
            )
            markdown_path.parent.mkdir(parents=True, exist_ok=True)
            markdown_path.write_text(markdown_content, encoding="utf-8")

        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE memory_items
                    SET title = ?, summary = ?, content_markdown = ?, importance = ?, updated_at = ?
                    WHERE id = ? AND deleted_at IS NULL
                    """,
                    (title, summary, content_markdown, importance, timestamp, memory_id),
                )
                self.replace_tags(connection, memory_id, tags)
                self.replace_chunks(connection, memory_id, content_markdown, timestamp)
                self.replace_entities(connection, memory_id, analysis.entities, timestamp)
                self.replace_insights(
                    connection,
                    memory_id,
                    [*analysis.action_items, *analysis.decisions],
                    timestamp,
                )

        return self.get(memory_id)

    def delete(self, memory_id: str) -> bool:
        current = self.get(memory_id)
        timestamp = utc_timestamp()

        with self.connect() as connection:
            with connection:
                cursor = connection.execute(
                    "UPDATE memory_items SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
                    (timestamp, timestamp, memory_id),
                )
                connection.execute("DELETE FROM memory_tags WHERE memory_id = ?", (memory_id,))
                connection.execute("DELETE FROM memory_chunks WHERE memory_id = ?", (memory_id,))
                connection.execute("DELETE FROM memory_entities WHERE memory_id = ?", (memory_id,))
                connection.execute("DELETE FROM memory_insights WHERE memory_id = ?", (memory_id,))

        if current.markdown_path:
            try:
                Path(current.markdown_path).unlink()
            except FileNotFoundError:
                pass

        return cursor.rowcount > 0

    def reindex(self) -> MemoryReindexResponse:
        self.initialize()
        reindexed = 0
        missing_markdown = 0
        timestamp = utc_timestamp()

        with self.connect() as connection:
            rows = connection.execute(
                "SELECT * FROM memory_items WHERE deleted_at IS NULL AND markdown_path IS NOT NULL"
            ).fetchall()

            with connection:
                for row in rows:
                    path = Path(row["markdown_path"])
                    if not path.exists():
                        missing_markdown += 1
                        continue

                    content = path.read_text(encoding="utf-8")
                    title, summary, body = parse_markdown(content, fallback_title=row["title"])
                    existing_tags = [
                        tag_row["tag"]
                        for tag_row in connection.execute(
                            "SELECT tag FROM memory_tags WHERE memory_id = ?",
                            (row["id"],),
                        ).fetchall()
                    ]
                    analysis = analyze_memory(
                        title=title,
                        summary=summary,
                        content_markdown=body,
                        source_type=row["source_type"],
                        memory_type=row["type"],
                        existing_tags=existing_tags,
                        existing_importance=row["importance"],
                    )
                    sanitized_markdown = self.render_markdown(
                        memory_id=row["id"],
                        memory_type=row["type"],
                        title=title,
                        summary=analysis.summary,
                        content_markdown=analysis.content_markdown,
                        source_type=row["source_type"],
                        source_id=row["source_id"],
                        source_uri=row["source_uri"],
                        importance=analysis.importance,
                        tags=list(analysis.tags),
                        created_at=row["created_at"],
                        updated_at=timestamp,
                    )
                    path.write_text(sanitized_markdown, encoding="utf-8")
                    connection.execute(
                        """
                        UPDATE memory_items
                        SET title = ?, summary = ?, content_markdown = ?, importance = ?, updated_at = ?
                        WHERE id = ?
                        """,
                        (
                            title,
                            analysis.summary,
                            analysis.content_markdown,
                            analysis.importance,
                            timestamp,
                            row["id"],
                        ),
                    )
                    self.replace_tags(connection, row["id"], list(analysis.tags))
                    self.replace_chunks(connection, row["id"], analysis.content_markdown, timestamp)
                    self.replace_entities(connection, row["id"], analysis.entities, timestamp)
                    self.replace_insights(
                        connection,
                        row["id"],
                        [*analysis.action_items, *analysis.decisions],
                        timestamp,
                    )
                    reindexed += 1

        return MemoryReindexResponse(reindexed=reindexed, missing_markdown=missing_markdown)

    def export(self) -> MemoryExportResponse:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM memory_items
                WHERE deleted_at IS NULL
                ORDER BY updated_at DESC
                """
            ).fetchall()
        return MemoryExportResponse(
            exported_at=utc_timestamp(),
            items=[self.row_to_item(row) for row in rows],
        )

    def generate_daily_summary(self, request: DailySummaryRequest) -> MemoryItem:
        target_date = request.date or utc_timestamp()[:10]
        items = self.items_for_date(target_date)
        title, summary, content, tags = build_daily_summary(target_date, items)
        existing = self.find_by_source("memory_pipeline_daily", target_date)
        payload = MemoryCreateRequest(
            type="daily_summary",
            title=title,
            summary=summary,
            content_markdown=content,
            source_type="memory_pipeline_daily",
            source_id=target_date,
            importance=4 if items else 2,
            tags=tags,
        )
        if existing:
            return self.update(
                existing.id,
                MemoryUpdateRequest(
                    title=payload.title,
                    summary=payload.summary,
                    content_markdown=payload.content_markdown,
                    importance=payload.importance,
                    tags=payload.tags,
                ),
            )
        return self.create(payload)

    def generate_project_summary(self, request: ProjectSummaryRequest) -> MemoryItem:
        project = request.project.strip()
        if not project:
            raise ValueError("Project name is required.")
        items = self.items_for_project(project)
        title, summary, content, tags = build_project_summary(project, items)
        source_id = slugify(project) or project
        existing = self.find_by_source("memory_pipeline_project", source_id)
        payload = MemoryCreateRequest(
            type="project_summary",
            title=title,
            summary=summary,
            content_markdown=content,
            source_type="memory_pipeline_project",
            source_id=source_id,
            importance=4 if items else 2,
            tags=tags,
        )
        if existing:
            return self.update(
                existing.id,
                MemoryUpdateRequest(
                    title=payload.title,
                    summary=payload.summary,
                    content_markdown=payload.content_markdown,
                    importance=payload.importance,
                    tags=payload.tags,
                ),
            )
        return self.create(payload)

    def row_to_item(self, row: sqlite3.Row, read_markdown: bool = False) -> MemoryItem:
        content_markdown = row["content_markdown"]
        markdown_path = row["markdown_path"]

        if read_markdown and markdown_path:
            path = Path(markdown_path)
            if path.exists():
                _title, _summary, content_markdown = parse_markdown(
                    path.read_text(encoding="utf-8"),
                    fallback_title=row["title"],
                )

        return MemoryItem(
            id=row["id"],
            type=row["type"],
            title=row["title"],
            summary=row["summary"],
            content_markdown=content_markdown,
            markdown_path=markdown_path,
            source_type=row["source_type"],
            source_id=row["source_id"],
            source_uri=row["source_uri"],
            importance=row["importance"],
            tags=self.tags_for(row["id"]),
            entities=self.entities_for(row["id"]),
            action_items=self.insights_for(row["id"], "action_item"),
            decisions=self.insights_for(row["id"], "decision"),
            created_at=row["created_at"],
            updated_at=row["updated_at"],
            deleted_at=row["deleted_at"],
        )

    def tags_for(self, memory_id: str) -> list[str]:
        with self.connect() as connection:
            rows = connection.execute(
                "SELECT tag FROM memory_tags WHERE memory_id = ? ORDER BY tag",
                (memory_id,),
            ).fetchall()
        return [row["tag"] for row in rows]

    def entities_for(self, memory_id: str) -> list[MemoryEntity]:
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM memory_entities
                WHERE memory_id = ?
                ORDER BY entity_type, name
                """,
                (memory_id,),
            ).fetchall()
        return [
            self.row_to_entity(row)
            for row in rows
        ]

    def insights_for(self, memory_id: str, insight_type: str) -> list[MemoryInsight]:
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM memory_insights
                WHERE memory_id = ? AND type = ?
                ORDER BY created_at, title
                """,
                (memory_id, insight_type),
            ).fetchall()
        return [
            self.row_to_insight(row)
            for row in rows
        ]

    @staticmethod
    def row_to_entity(row: sqlite3.Row) -> MemoryEntity:
        return MemoryEntity(
            id=row["id"],
            memory_id=row["memory_id"],
            memory_title=optional_row_value(row, "memory_title"),
            source_type=optional_row_value(row, "source_type"),
            source_id=optional_row_value(row, "source_id"),
            source_uri=optional_row_value(row, "source_uri"),
            name=row["name"],
            entity_type=row["entity_type"],
            source_text=row["source_text"],
            created_at=row["created_at"],
        )

    @staticmethod
    def row_to_insight(row: sqlite3.Row) -> MemoryInsight:
        return MemoryInsight(
            id=row["id"],
            memory_id=row["memory_id"],
            memory_title=optional_row_value(row, "memory_title"),
            source_type=optional_row_value(row, "source_type"),
            source_id=optional_row_value(row, "source_id"),
            source_uri=optional_row_value(row, "source_uri"),
            type=row["type"],
            title=row["title"],
            detail=row["detail"],
            status=row["status"],
            due_at=row["due_at"],
            created_at=row["created_at"],
        )

    def items_for_date(self, target_date: str) -> list[MemoryItem]:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM memory_items
                WHERE deleted_at IS NULL
                  AND created_at LIKE ?
                  AND type NOT IN ('daily_summary', 'project_summary')
                  AND source_type NOT LIKE 'memory_pipeline_%'
                ORDER BY updated_at DESC
                LIMIT 80
                """,
                (f"{target_date}%",),
            ).fetchall()
        return [self.row_to_item(row) for row in rows]

    def items_for_project(self, project: str) -> list[MemoryItem]:
        self.initialize()
        pattern = f"%{project.strip().lower()}%"
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM memory_items
                WHERE deleted_at IS NULL
                  AND type NOT IN ('daily_summary', 'project_summary')
                  AND source_type NOT LIKE 'memory_pipeline_%'
                  AND (
                    lower(title) LIKE ?
                    OR lower(summary) LIKE ?
                    OR lower(content_markdown) LIKE ?
                    OR id IN (
                      SELECT memory_id FROM memory_tags WHERE lower(tag) LIKE ?
                    )
                    OR id IN (
                      SELECT memory_id FROM memory_entities WHERE lower(name) LIKE ?
                    )
                  )
                ORDER BY updated_at DESC
                LIMIT 100
                """,
                (pattern, pattern, pattern, pattern, pattern),
            ).fetchall()
        return [self.row_to_item(row) for row in rows]

    def find_by_source(self, source_type: str, source_id: str) -> MemoryItem | None:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                """
                SELECT * FROM memory_items
                WHERE deleted_at IS NULL
                  AND source_type = ?
                  AND source_id = ?
                ORDER BY updated_at DESC
                LIMIT 1
                """,
                (source_type, source_id),
            ).fetchone()
        return self.row_to_item(row) if row else None

    def list_entities(
        self,
        query: str | None = None,
        *,
        source_type: str | None = None,
        source_id: str | None = None,
        date: str | None = None,
        limit: int = 100,
    ) -> MemoryEntityListResponse:
        self.initialize()
        limit = max(1, min(limit, 200))
        query_value = (query or "").strip()
        source_type_value = (source_type or "").strip()
        source_id_value = (source_id or "").strip()
        date_value = (date or "").strip()
        where = "items.deleted_at IS NULL"
        params: list[object] = []

        if query_value:
            where += " AND (lower(entities.name) LIKE ? OR lower(entities.entity_type) LIKE ?)"
            pattern = f"%{query_value.lower()}%"
            params.extend([pattern, pattern])
        if source_type_value:
            where += " AND items.source_type = ?"
            params.append(source_type_value)
        if source_id_value:
            where += " AND items.source_id = ?"
            params.append(source_id_value)
        if date_value:
            where += " AND items.created_at LIKE ?"
            params.append(f"{date_value}%")

        with self.connect() as connection:
            rows = connection.execute(
                f"""
                SELECT
                  entities.*,
                  items.title AS memory_title,
                  items.source_type AS source_type,
                  items.source_id AS source_id,
                  items.source_uri AS source_uri
                FROM memory_entities AS entities
                JOIN memory_items AS items ON items.id = entities.memory_id
                WHERE {where}
                ORDER BY entities.created_at DESC, entities.name
                LIMIT ?
                """,
                [*params, limit],
            ).fetchall()
            total = connection.execute(
                f"""
                SELECT COUNT(*) AS count
                FROM memory_entities AS entities
                JOIN memory_items AS items ON items.id = entities.memory_id
                WHERE {where}
                """,
                params,
            ).fetchone()["count"]

        return MemoryEntityListResponse(
            items=[self.row_to_entity(row) for row in rows],
            total=total,
            query=query_value or None,
            source_type=source_type_value or None,
            source_id=source_id_value or None,
            date=date_value or None,
        )

    def list_insights(
        self,
        *,
        query: str | None = None,
        insight_type: MemoryInsightType | None = None,
        status: str | None = None,
        source_type: str | None = None,
        source_id: str | None = None,
        date: str | None = None,
        limit: int = 100,
    ) -> MemoryInsightListResponse:
        self.initialize()
        limit = max(1, min(limit, 200))
        query_value = (query or "").strip()
        status_value = (status or "").strip()
        source_type_value = (source_type or "").strip()
        source_id_value = (source_id or "").strip()
        date_value = (date or "").strip()
        where = "items.deleted_at IS NULL"
        params: list[object] = []

        if insight_type:
            where += " AND insights.type = ?"
            params.append(insight_type)
        if query_value:
            where += """
              AND (
                lower(insights.title) LIKE ?
                OR lower(insights.detail) LIKE ?
                OR lower(items.title) LIKE ?
                OR lower(items.summary) LIKE ?
                OR lower(COALESCE(items.source_id, '')) LIKE ?
              )
            """
            pattern = f"%{query_value.lower()}%"
            params.extend([pattern, pattern, pattern, pattern, pattern])
        if status_value:
            where += " AND insights.status = ?"
            params.append(status_value)
        if source_type_value:
            where += " AND items.source_type = ?"
            params.append(source_type_value)
        if source_id_value:
            where += " AND items.source_id = ?"
            params.append(source_id_value)
        if date_value:
            where += " AND items.created_at LIKE ?"
            params.append(f"{date_value}%")

        with self.connect() as connection:
            rows = connection.execute(
                f"""
                SELECT
                  insights.*,
                  items.title AS memory_title,
                  items.source_type AS source_type,
                  items.source_id AS source_id,
                  items.source_uri AS source_uri
                FROM memory_insights AS insights
                JOIN memory_items AS items ON items.id = insights.memory_id
                WHERE {where}
                ORDER BY insights.created_at DESC, insights.title
                LIMIT ?
                """,
                [*params, limit],
            ).fetchall()
            total = connection.execute(
                f"""
                SELECT COUNT(*) AS count
                FROM memory_insights AS insights
                JOIN memory_items AS items ON items.id = insights.memory_id
                WHERE {where}
                """,
                params,
            ).fetchone()["count"]

        return MemoryInsightListResponse(
            items=[self.row_to_insight(row) for row in rows],
            total=total,
            query=query_value or None,
            type=insight_type,
            status=status_value or None,
            source_type=source_type_value or None,
            source_id=source_id_value or None,
            date=date_value or None,
        )

    def require_vault_root(self) -> Path:
        settings = self.core_store.read_settings()
        if not settings.vault_path:
            raise ValueError("Complete onboarding and choose a vault before writing memory.")

        vault_root = Path(settings.vault_path)
        create_vault_template(vault_root)
        return vault_root

    @staticmethod
    def markdown_path(vault_root: Path, folder: str, title: str, memory_id: str) -> Path:
        slug = slugify(title) or "memory"
        return vault_root / folder / f"{slug}-{memory_id[-8:]}.md"

    @staticmethod
    def default_content(title: str, summary: str) -> str:
        return f"# {title}\n\n{summary}\n"

    @staticmethod
    def render_markdown(
        memory_id: str,
        memory_type: str,
        title: str,
        summary: str,
        content_markdown: str,
        source_type: str,
        source_id: str | None,
        source_uri: str | None,
        importance: int,
        tags: list[str],
        created_at: str,
        updated_at: str,
    ) -> str:
        frontmatter = {
            "id": memory_id,
            "type": memory_type,
            "source_type": source_type,
            "source_id": source_id,
            "source_uri": source_uri,
            "importance": importance,
            "tags": tags,
            "created_at": created_at,
            "updated_at": updated_at,
        }
        lines = ["---"]
        for key, value in frontmatter.items():
            lines.append(f"{key}: {json.dumps(value)}")
        lines.extend(["---", "", f"# {title}", "", f"> {summary}", "", content_markdown.strip(), ""])
        return "\n".join(lines)

    @staticmethod
    def replace_tags(connection: sqlite3.Connection, memory_id: str, tags: list[str]) -> None:
        connection.execute("DELETE FROM memory_tags WHERE memory_id = ?", (memory_id,))
        for tag in sorted({tag.strip().lower() for tag in tags if tag.strip()}):
            connection.execute(
                "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?, ?)",
                (memory_id, tag),
            )

    @staticmethod
    def replace_chunks(
        connection: sqlite3.Connection, memory_id: str, content_markdown: str, created_at: str
    ) -> None:
        connection.execute("DELETE FROM memory_chunks WHERE memory_id = ?", (memory_id,))
        chunks = chunk_text(content_markdown)
        for index, chunk in enumerate(chunks):
            connection.execute(
                """
                INSERT INTO memory_chunks (
                  id, memory_id, chunk_text, chunk_index, token_estimate,
                  embedding_model, embedding_ref, created_at
                )
                VALUES (?, ?, ?, ?, ?, NULL, NULL, ?)
                """,
                (
                    f"chunk_{uuid.uuid4().hex}",
                    memory_id,
                    chunk,
                    index,
                    max(1, len(chunk.split())),
                    created_at,
                ),
            )

    @staticmethod
    def replace_entities(
        connection: sqlite3.Connection,
        memory_id: str,
        entities,
        created_at: str,
    ) -> None:
        connection.execute("DELETE FROM memory_entities WHERE memory_id = ?", (memory_id,))
        for entity in entities:
            connection.execute(
                """
                INSERT INTO memory_entities (
                  id, memory_id, name, entity_type, source_text, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (
                    f"entity_{uuid.uuid4().hex}",
                    memory_id,
                    entity.name,
                    entity.entity_type,
                    entity.source_text,
                    created_at,
                ),
            )

    @staticmethod
    def replace_insights(
        connection: sqlite3.Connection,
        memory_id: str,
        insights,
        created_at: str,
    ) -> None:
        connection.execute("DELETE FROM memory_insights WHERE memory_id = ?", (memory_id,))
        for insight in insights:
            connection.execute(
                """
                INSERT INTO memory_insights (
                  id, memory_id, type, title, detail, status, due_at, created_at
                )
                VALUES (?, ?, ?, ?, ?, 'open', ?, ?)
                """,
                (
                    f"insight_{uuid.uuid4().hex}",
                    memory_id,
                    insight.insight_type,
                    insight.title,
                    insight.detail,
                    insight.due_at,
                    created_at,
                ),
            )


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9]+", "-", value.strip().lower()).strip("-")
    return slug[:72]


def optional_row_value(row: sqlite3.Row, key: str) -> str | None:
    return row[key] if key in row.keys() else None


def folder_for_memory(memory_type: str, source_type: str) -> str:
    if memory_type == "connector_summary":
        return CONNECTOR_SOURCE_TO_FOLDER.get(source_type, TYPE_TO_FOLDER["connector_summary"])
    return TYPE_TO_FOLDER.get(memory_type, "Inbox")


def chunk_text(content: str, size: int = 900) -> list[str]:
    normalized = content.strip()
    if not normalized:
        return []
    return [normalized[index : index + size] for index in range(0, len(normalized), size)]


def parse_markdown(content: str, fallback_title: str) -> tuple[str, str, str]:
    body = strip_frontmatter(content).strip()
    title = fallback_title
    lines = body.splitlines()

    for line in lines:
        if line.startswith("# "):
            title = line[2:].strip() or fallback_title
            break

    summary = ""
    for line in lines:
        stripped = line.strip()
        if stripped.startswith(">"):
            summary = stripped.lstrip("> ").strip()
            break
        if stripped and not stripped.startswith("#"):
            summary = stripped[:240]
            break

    return title, summary or title, body


def strip_frontmatter(content: str) -> str:
    if not content.startswith("---"):
        return content

    parts = content.split("---", 2)
    if len(parts) == 3:
        return parts[2]
    return content
