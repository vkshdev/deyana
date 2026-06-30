from __future__ import annotations

import json
import socket
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path, ollama_endpoint: str) -> TestClient:
    settings = CoreSettings(
        data_dir=tmp_path / "data",
        log_dir=tmp_path / "logs",
        ollama_endpoint=ollama_endpoint,
    )
    return TestClient(create_app(RuntimeState(settings)))


class FakeOllama:
    def __init__(self, models: list[dict[str, Any]] | None = None) -> None:
        self.models = models or [
            {"name": "qwen3:1.7b", "size": 1_400_000_000},
            {"name": "all-minilm:latest", "size": 45_000_000},
        ]
        self.requests: list[dict[str, Any]] = []
        self.server: ThreadingHTTPServer | None = None
        self.thread: threading.Thread | None = None
        self.endpoint = ""

    def __enter__(self) -> "FakeOllama":
        parent = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:
                if self.path == "/api/tags":
                    self.send_json(200, {"models": parent.models})
                    return
                self.send_json(404, {"error": "not found"})

            def do_POST(self) -> None:
                if self.path != "/api/generate":
                    self.send_json(404, {"error": "not found"})
                    return

                length = int(self.headers.get("content-length", "0"))
                payload = json.loads(self.rfile.read(length).decode("utf-8"))
                parent.requests.append(payload)

                model = payload.get("model")
                installed = {item["name"] for item in parent.models}
                if model not in installed:
                    self.send_json(404, {"error": f"model {model} not found"})
                    return

                prompt = payload.get("prompt", "")
                response = "DEYANA_READY" if "DEYANA_READY" in prompt else "Local answer"
                self.send_json(200, {"model": model, "response": response})

            def log_message(self, _format: str, *_args: object) -> None:
                return

            def send_json(self, status: int, payload: dict[str, Any]) -> None:
                body = json.dumps(payload).encode("utf-8")
                self.send_response(status)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        port = self.server.server_address[1]
        self.endpoint = f"http://127.0.0.1:{port}"
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        if self.server:
            self.server.shutdown()
            self.server.server_close()
        if self.thread:
            self.thread.join(timeout=2)


def test_model_status_detects_ollama_and_selected_models(tmp_path) -> None:
    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        response = client.get("/models/status")
        core_status = client.get("/status")

    assert response.status_code == 200
    body = response.json()
    assert body["status"] == "available"
    assert body["selectedChatModel"] == "qwen3:1.7b"
    assert body["selectedEmbeddingModel"] == "all-minilm:latest"
    assert body["chatModelAvailable"] is True
    assert body["embeddingModelAvailable"] is True
    assert body["think"] is False
    assert {model["name"] for model in body["availableModels"]} == {
        "qwen3:1.7b",
        "all-minilm:latest",
    }

    assert core_status.status_code == 200
    dependencies = {item["name"]: item for item in core_status.json()["dependencies"]}
    assert dependencies["ollama"]["status"] == "available"
    assert core_status.json()["featureFlags"]["models"] is True
    assert core_status.json()["featureFlags"]["chat"] is True


def test_model_selection_persists_and_reports_missing_model(tmp_path) -> None:
    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        selection = client.patch(
            "/models/selection",
            json={"chatModel": "llama3.2:1b", "profile": "low_spec"},
        )
        settings = client.get("/settings")
        test_prompt = client.post("/model/test", json={"prompt": "Reply with ready"})

    assert selection.status_code == 200
    assert selection.json()["status"]["status"] == "missing"
    assert selection.json()["settings"]["selectedChatModel"] == "llama3.2:1b"
    assert settings.json()["selectedChatModel"] == "llama3.2:1b"
    assert test_prompt.status_code == 409
    assert "ollama pull llama3.2:1b" in test_prompt.json()["detail"]


def test_model_test_prompt_uses_ollama_with_thinking_disabled(tmp_path) -> None:
    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        response = client.post(
            "/model/test",
            json={"prompt": "Reply with exactly: DEYANA_READY"},
        )

    assert response.status_code == 200
    assert response.json()["response"] == "DEYANA_READY"
    assert ollama.requests
    request = ollama.requests[-1]
    assert request["model"] == "qwen3:1.7b"
    assert request["stream"] is False
    assert request["think"] is False
    assert "local-first private desktop AI assistant" in request["prompt"]


def test_chat_message_uses_local_model_and_stores_history(tmp_path) -> None:
    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        response = client.post("/chat/message", json={"content": "Give me a local reply."})
        history = client.get("/chat/history")
        deleted = client.delete("/chat/history")
        empty_history = client.get("/chat/history")

    assert response.status_code == 200
    body = response.json()
    assert body["model"] == "qwen3:1.7b"
    assert body["userMessage"]["content"] == "Give me a local reply."
    assert body["assistantMessage"]["content"] == "Local answer"
    assert body["sources"] == []
    assert body["retrieval"]["retrieved"] == 0
    assert ollama.requests[-1]["think"] is False

    assert history.status_code == 200
    assert [message["role"] for message in history.json()["messages"]] == ["user", "assistant"]
    assert deleted.json()["deleted"] == 2
    assert empty_history.json()["messages"] == []


def test_chat_agent_retrieves_memory_and_persists_source_references(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        complete_onboarding(client, vault_path)
        created = client.post(
            "/memory",
            json={
                "type": "decision",
                "title": "Local memory storage decision",
                "summary": "Deyana stores private memory locally by default.",
                "contentMarkdown": (
                    "Use SQLite for structured private memory. "
                    "Mirror compressed summaries into the user-owned Markdown vault. "
                    "Keep the private assistant local-first."
                ),
                "tags": ["architecture", "privacy"],
            },
        ).json()

        response = client.post(
            "/chat/message",
            json={"content": "What did we decide about local memory storage?"},
        )
        history = client.get("/chat/history")

    assert response.status_code == 200
    body = response.json()
    assert body["retrieval"]["retrieved"] == 1
    assert body["retrieval"]["compressedCharacters"] > 0
    assert body["retrieval"]["contextTokensEstimate"] > 0
    assert body["sources"][0]["id"] == created["id"]
    assert body["sources"][0]["label"] == "S1"
    assert body["sources"][0]["title"] == "Local memory storage decision"
    assert body["sources"][0]["markdownPath"].endswith(".md")
    assert "locally" in body["sources"][0]["snippet"].lower()
    assert "Sources: [S1] Local memory storage decision" in body["assistantMessage"]["content"]
    assert body["assistantMessage"]["sourceReferences"][0]["id"] == created["id"]

    prompt = ollama.requests[-1]["prompt"]
    assert "LOCAL MEMORY CONTEXT" in prompt
    assert "[S1] Local memory storage decision" in prompt
    assert "Compressed snippet:" in prompt
    assert "Cite memory claims inline" in prompt
    assert len(prompt) < 4200

    assistant_from_history = history.json()["messages"][-1]
    assert assistant_from_history["sourceReferences"][0]["title"] == "Local memory storage decision"


def test_chat_agent_can_skip_memory_retrieval(tmp_path) -> None:
    vault_path = tmp_path / "Vault"

    with FakeOllama() as ollama, make_client(tmp_path, ollama.endpoint) as client:
        complete_onboarding(client, vault_path)
        client.post(
            "/memory",
            json={
                "type": "note",
                "title": "Roadmap reminder",
                "summary": "Phase 6 answers from memory.",
                "contentMarkdown": "This should not be retrieved when useMemory is false.",
            },
        )
        response = client.post(
            "/chat/message",
            json={"content": "What does the roadmap reminder say?", "useMemory": False},
        )

    assert response.status_code == 200
    assert response.json()["sources"] == []
    assert response.json()["retrieval"]["retrieved"] == 0
    assert "No matching local memory was retrieved." in ollama.requests[-1]["prompt"]


def test_offline_ollama_status_and_chat_error_are_user_friendly(tmp_path) -> None:
    endpoint = f"http://127.0.0.1:{unused_port()}"

    with make_client(tmp_path, endpoint) as client:
        status = client.get("/models/status")
        chat = client.post("/chat/message", json={"content": "Hello"})

    assert status.status_code == 200
    assert status.json()["status"] == "offline"
    assert "Start Ollama" in status.json()["message"]
    assert chat.status_code == 503
    assert "Start Ollama" in chat.json()["detail"]


def unused_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def complete_onboarding(client: TestClient, vault_path) -> None:
    response = client.post(
        "/onboarding/complete",
        json={
            "privacyMode": "local_only",
            "modelProfile": "low_spec",
            "vaultPath": str(vault_path),
        },
    )
    assert response.status_code == 200
