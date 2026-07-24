import pytest
from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings

@pytest.fixture
def runtime_state(tmp_path):
    settings = CoreSettings(
        data_dir=tmp_path,
        host="127.0.0.1",
        port=1420,
        ollama_endpoint="http://127.0.0.1:11434"
    )
    # We initialize RuntimeState directly to avoid relying on global state
    runtime = RuntimeState(settings)
    yield runtime

@pytest.fixture
def client(runtime_state):
    app = create_app(runtime=runtime_state)
    with TestClient(app) as test_client:
        yield test_client

def test_triage_incoming_enqueue(client):
    response = client.post(
        "/api/triage/incoming",
        json={"platform": "discord", "sender": "test_user", "content": "hello"},
        headers={"x-deyana-client": "true"}
    )
    assert response.status_code == 200
    assert response.json() == {"status": "enqueued"}

def test_triage_pending(client, runtime_state):
    # Ensure there are no pending messages initially
    response = client.get("/api/triage/pending")
    assert response.status_code == 200
    assert response.json() == []

def test_triage_flow(client, runtime_state):
    # Enqueue a message
    client.post(
        "/api/triage/incoming",
        json={"platform": "test_platform", "sender": "bot", "content": "test_message"},
        headers={"x-deyana-client": "true"}
    )
    
    # Normally the daemon would pick this up and write to DB
    # We simulate the daemon's DB write directly to test the /pending and /resolve endpoints
    from deyana_core.triage import IncomingTriageRequest
    req = IncomingTriageRequest(platform="test_platform", sender="bot", content="test_message")
    import asyncio
    asyncio.run(runtime_state.triage_daemon._save_triage_result(req, "URGENT", "Mock draft"))

    # Fetch pending
    res = client.get("/api/triage/pending")
    assert res.status_code == 200
    data = res.json()
    assert len(data) >= 1
    
    # Find our message
    msg_id = None
    for d in data:
        if d["content"] == "test_message":
            msg_id = d["id"]
            break
    
    assert msg_id is not None

    # Resolve message
    res_resolve = client.post(f"/api/triage/{msg_id}/resolve", json={"status": "approved"})
    assert res_resolve.status_code == 200

    # Ensure it is removed from pending
    res_pending = client.get("/api/triage/pending")
    assert len(res_pending.json()) == 0
