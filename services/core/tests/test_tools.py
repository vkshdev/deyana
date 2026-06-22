from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def complete_onboarding(client: TestClient, tmp_path) -> None:
    vault_path = tmp_path / "vault"
    assert client.post("/vault/select", json={"path": str(vault_path)}).status_code == 200
    assert (
        client.post(
            "/onboarding/complete",
            json={
                "privacyMode": "local_only",
                "modelProfile": "low_spec",
                "vaultPath": str(vault_path),
            },
        ).status_code
        == 200
    )


def test_tools_are_listed_and_feature_flagged(tmp_path) -> None:
    with make_client(tmp_path) as client:
        tools = client.get("/tools")
        status = client.get("/status")

    assert tools.status_code == 200
    tool_ids = {tool["toolId"] for tool in tools.json()["tools"]}
    assert {
        "web_search",
        "fetch_page",
        "read_file",
        "git_status",
        "git_diff",
        "commit_message",
        "code_task",
        "day_planner",
    }.issubset(tool_ids)
    assert status.json()["featureFlags"]["tools"] is True
    assert status.json()["featureFlags"]["dayPlanner"] is True


def test_public_and_local_tools_are_permission_gated(tmp_path) -> None:
    file_path = tmp_path / "vault" / "note.md"
    file_path.parent.mkdir(parents=True)
    file_path.write_text("# Local note\n\nPrivate local content.", encoding="utf-8")

    with make_client(tmp_path) as client:
        web = client.post("/tools/web-search", json={"query": "public docs"})
        blocked_file = client.post(
            "/tools/read-file",
            json={"path": str(file_path), "allowedRoot": str(tmp_path / "other"), "userApproved": True},
        )
        file_result = client.post(
            "/tools/read-file",
            json={"path": str(file_path), "allowedRoot": str(file_path.parent), "userApproved": True},
        )

    assert web.status_code == 200
    assert web.json()["status"] == "permission_required"
    assert web.json()["permissionRequired"] is True
    assert blocked_file.status_code == 400
    assert file_result.status_code == 200
    assert file_result.json()["status"] == "completed"
    assert "Private local content" in file_result.json()["content"]


def test_code_task_is_proposal_only_and_day_planner_uses_local_actions(tmp_path) -> None:
    with make_client(tmp_path) as client:
        complete_onboarding(client, tmp_path)
        client.post(
            "/memory",
            json={
                "title": "Launch follow up",
                "contentMarkdown": "Next step: update launch checklist by 2026-06-30.",
            },
        )
        code_without_approval = client.post("/tools/code/task", json={"goal": "Explain this module"})
        code = client.post(
            "/tools/code/task",
            json={"goal": "Explain this module", "context": "def add(a, b): return a + b", "userApproved": True},
        )
        plan = client.post("/tools/day-planner", json={"focus": ["Ship Phase 11"]})

    assert code_without_approval.status_code == 200
    assert code_without_approval.json()["status"] == "permission_required"
    assert code.status_code == 200
    assert code.json()["appliesChanges"] is False
    assert "Apply edits only after explicit user confirmation" in code.json()["content"]
    assert plan.status_code == 200
    assert "Ship Phase 11" in plan.json()["content"]
    assert "update launch checklist" in plan.json()["content"]
