from __future__ import annotations

import sqlite3
from pathlib import Path

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path):
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def complete_onboarding(client: TestClient, vault_path: Path) -> None:
    response = client.post(
        "/onboarding/complete",
        json={
            "privacyMode": "local_only",
            "modelProfile": "low_spec",
            "vaultPath": str(vault_path),
        },
    )
    assert response.status_code == 200


def test_memory_create_writes_sqlite_and_markdown(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with make_client(tmp_path) as client:
        complete_onboarding(client, vault_path)
        response = client.post(
            "/memory",
            json={
                "type": "note",
                "title": "Shipping checklist",
                "summary": "Release gate notes",
                "contentMarkdown": "Check tests, build, and native smoke.",
                "tags": ["release", "phase4"],
            },
        )

    assert response.status_code == 200
    body = response.json()
    assert body["title"] == "Shipping checklist"
    assert body["markdownPath"]
    assert Path(body["markdownPath"]).is_file()
    markdown = Path(body["markdownPath"]).read_text(encoding="utf-8")
    assert markdown.startswith("---")
    assert 'source_type: "manual"' in markdown
    assert "importance:" in markdown

    database_path = tmp_path / "data" / "memory.sqlite3"
    assert database_path.is_file()
    with sqlite3.connect(database_path) as connection:
        row = connection.execute(
            "SELECT title, summary FROM memory_items WHERE id = ?",
            (body["id"],),
        ).fetchone()
    assert row == ("Shipping checklist", "Release gate notes")


def test_memory_pipeline_extracts_actions_decisions_entities_and_tags(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with make_client(tmp_path) as client:
        complete_onboarding(client, vault_path)
        response = client.post(
            "/memory",
            json={
                "type": "note",
                "title": "DEYANA launch review",
                "sourceId": "phase10-launch-review",
                "contentMarkdown": (
                    "Alex approved using local summaries for DEYANA. "
                    "Need to follow up with founder@example.com by 2026-06-30. "
                    "Track repo deyana/local-assistant."
                ),
            },
        )
        created_body = response.json()
        created_date = created_body["createdAt"][:10]
        search = client.get("/memory", params={"query": "founder@example.com"})
        entities = client.get("/memory/entities", params={"query": "founder"})
        filtered_entities = client.get(
            "/memory/entities",
            params={
                "query": "founder",
                "sourceType": "manual",
                "sourceId": "phase10-launch-review",
                "date": created_date,
            },
        )
        actions = client.get("/memory/insights", params={"type": "action_item"})
        filtered_actions = client.get(
            "/memory/insights",
            params={
                "query": "founder@example.com",
                "type": "action_item",
                "status": "open",
                "sourceType": "manual",
                "sourceId": "phase10-launch-review",
                "date": created_date,
            },
        )
        decisions = client.get("/memory/insights", params={"type": "decision"})
        invalid_insight_type = client.get("/memory/insights", params={"type": "reminder"})

    assert response.status_code == 200
    body = response.json()
    assert body["summary"]
    assert body["importance"] >= 4
    assert "action-item" in body["tags"]
    assert "decision" in body["tags"]
    assert body["actionItems"]
    assert body["decisions"]
    assert any(entity["name"] == "founder@example.com" for entity in body["entities"])
    assert "## Extracted action items" in body["contentMarkdown"]
    assert search.status_code == 200
    assert search.json()["total"] == 1
    assert entities.status_code == 200
    assert entities.json()["total"] >= 1
    assert entities.json()["items"][0]["memoryTitle"] == "DEYANA launch review"
    assert entities.json()["items"][0]["sourceType"] == "manual"
    assert filtered_entities.status_code == 200
    assert filtered_entities.json()["sourceType"] == "manual"
    assert filtered_entities.json()["sourceId"] == "phase10-launch-review"
    assert filtered_entities.json()["date"] == body["createdAt"][:10]
    assert filtered_entities.json()["total"] >= 1
    assert actions.status_code == 200
    assert actions.json()["items"]
    assert actions.json()["items"][0]["memoryTitle"] == "DEYANA launch review"
    assert filtered_actions.status_code == 200
    assert filtered_actions.json()["query"] == "founder@example.com"
    assert filtered_actions.json()["type"] == "action_item"
    assert filtered_actions.json()["status"] == "open"
    assert filtered_actions.json()["sourceType"] == "manual"
    assert filtered_actions.json()["sourceId"] == "phase10-launch-review"
    assert filtered_actions.json()["date"] == body["createdAt"][:10]
    assert filtered_actions.json()["items"]
    assert decisions.status_code == 200
    assert decisions.json()["items"]
    assert invalid_insight_type.status_code == 422


def test_manual_markdown_edit_can_be_reindexed_and_searched(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with make_client(tmp_path) as client:
        complete_onboarding(client, vault_path)
        created = client.post(
            "/memory",
            json={
                "type": "decision",
                "title": "Original title",
                "summary": "Original summary",
                "contentMarkdown": "Original body",
            },
        ).json()
        markdown_path = Path(created["markdownPath"])
        markdown_path.write_text(
            (
                "---\nid: edited\n---\n\n# Edited decision\n\n> Manual vault edit\n\n"
                "Updated hand-edited memory body.\n\n"
                "## Extracted action items\n\n"
                "- Old stale action should be removed.\n\n"
                "## Notes\n\n"
                "Keep this user-authored section.\n"
            ),
            encoding="utf-8",
        )

        reindex = client.post("/memory/reindex")
        search = client.get("/memory", params={"query": "hand-edited"})
        fetched = client.get(f"/memory/{created['id']}")

    assert reindex.status_code == 200
    assert reindex.json()["reindexed"] == 1
    assert search.status_code == 200
    assert search.json()["total"] == 1
    assert fetched.json()["title"] == "Edited decision"
    assert "Updated hand-edited memory body." in fetched.json()["contentMarkdown"]
    assert "Old stale action should be removed." not in fetched.json()["contentMarkdown"]
    assert "Keep this user-authored section." in fetched.json()["contentMarkdown"]


def test_memory_export_and_delete(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with make_client(tmp_path) as client:
        complete_onboarding(client, vault_path)
        created = client.post(
            "/memory",
            json={
                "type": "action_item",
                "title": "Delete me",
                "summary": "Temporary memory",
                "contentMarkdown": "This memory should disappear.",
            },
        ).json()
        export_response = client.get("/memory/export")
        delete_response = client.delete(f"/memory/{created['id']}")
        list_response = client.get("/memory")
        get_deleted = client.get(f"/memory/{created['id']}")

    assert export_response.status_code == 200
    assert export_response.json()["items"][0]["id"] == created["id"]
    assert delete_response.status_code == 200
    assert delete_response.json()["deleted"] is True
    assert list_response.json()["total"] == 0
    assert get_deleted.status_code == 404
    assert not Path(created["markdownPath"]).exists()


def test_daily_and_project_summaries_are_generated_as_memory(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with make_client(tmp_path) as client:
        complete_onboarding(client, vault_path)
        client.post(
            "/memory",
            json={
                "type": "note",
                "title": "Project Cipher decision",
                "summary": "Cipher should keep local-only memory.",
                "contentMarkdown": "Decision: choose local-only memory. Next step: update Cipher docs.",
                "tags": ["cipher"],
            },
        )
        daily = client.post("/memory/summaries/daily", json={})
        project = client.post("/memory/summaries/project", json={"project": "Cipher"})

    assert daily.status_code == 200
    assert daily.json()["type"] == "daily_summary"
    assert daily.json()["markdownPath"]
    assert Path(daily.json()["markdownPath"]).is_file()
    assert "### Open action items" in daily.json()["contentMarkdown"]
    assert "update Cipher docs" in daily.json()["contentMarkdown"]
    assert "### Decisions" in daily.json()["contentMarkdown"]
    assert "choose local-only memory" in daily.json()["contentMarkdown"]

    assert project.status_code == 200
    assert project.json()["type"] == "project_summary"
    assert "Cipher" in project.json()["title"]
    assert project.json()["markdownPath"]
    assert Path(project.json()["markdownPath"]).is_file()
    assert "### Open action items" in project.json()["contentMarkdown"]
    assert "### Decisions" in project.json()["contentMarkdown"]
    assert "manual" in Path(project.json()["markdownPath"]).read_text(encoding="utf-8")


def test_memory_create_requires_vault(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.post(
            "/memory",
            json={"title": "No vault", "summary": "Should fail"},
        )

    assert response.status_code == 400
    assert "vault" in response.json()["detail"].lower()
