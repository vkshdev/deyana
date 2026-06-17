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
    assert Path(body["markdownPath"]).read_text(encoding="utf-8").startswith("---")

    database_path = tmp_path / "data" / "memory.sqlite3"
    assert database_path.is_file()
    with sqlite3.connect(database_path) as connection:
        row = connection.execute(
            "SELECT title, summary FROM memory_items WHERE id = ?",
            (body["id"],),
        ).fetchone()
    assert row == ("Shipping checklist", "Release gate notes")


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
            "---\nid: edited\n---\n\n# Edited decision\n\n> Manual vault edit\n\nUpdated hand-edited memory body.\n",
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


def test_memory_create_requires_vault(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.post(
            "/memory",
            json={"title": "No vault", "summary": "Should fail"},
        )

    assert response.status_code == 400
    assert "vault" in response.json()["detail"].lower()
