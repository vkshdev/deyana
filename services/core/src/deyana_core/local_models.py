from __future__ import annotations

import json
import socket
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from threading import Lock
from typing import Any

from .models import (
    AppSettings,
    LocalModelInfo,
    LocalModelStatusResponse,
    ModelProfile,
    ModelSelectionRequest,
    ModelSelectionResponse,
)
from .runtime_time import utc_timestamp
from .storage import CoreStore

DEFAULT_CHAT_MODEL = "qwen3:1.7b"
DEFAULT_EMBEDDING_MODEL = "all-minilm:latest"
MAX_PARALLEL_MODEL_JOBS = 1
THINK_FOR_NORMAL_CHAT = False

PROFILE_CHAT_MODELS: dict[ModelProfile, list[str]] = {
    "low_spec": ["qwen3:1.7b", "llama3.2:1b"],
    "balanced": ["qwen3:1.7b", "llama3.2:3b", "llama3.2:1b"],
    "power": ["qwen3:1.7b", "llama3.2:3b", "qwen2.5-coder:3b"],
}

PROFILE_EMBEDDING_MODELS: dict[ModelProfile, list[str]] = {
    "low_spec": ["all-minilm:latest"],
    "balanced": ["all-minilm:latest", "nomic-embed-text:latest"],
    "power": ["all-minilm:latest", "nomic-embed-text:latest"],
}

MODEL_DETAILS: dict[str, str] = {
    "qwen3:1.7b": "Default low-spec local chat model for this 8 GB laptop.",
    "llama3.2:1b": "Fallback local chat model when qwen3:1.7b is unavailable.",
    "llama3.2:3b": "Optional stronger local chat model after local latency testing.",
    "qwen2.5-coder:3b": "Optional coding model for future benchmarked workflows.",
    "all-minilm:latest": "Default tiny local embedding model for future retrieval.",
    "nomic-embed-text:latest": "Optional stronger local embedding model.",
}


class OllamaProviderError(RuntimeError):
    pass


class OllamaUnavailableError(OllamaProviderError):
    pass


class OllamaModelMissingError(OllamaProviderError):
    def __init__(self, model: str) -> None:
        super().__init__(
            f"Selected model '{model}' is not installed. Run `ollama pull {model}` or choose an installed model."
        )
        self.model = model


@dataclass(frozen=True)
class OllamaInstalledModel:
    name: str
    size_bytes: int | None


@dataclass(frozen=True)
class ModelGeneration:
    model: str
    response: str
    latency_ms: int


class OllamaClient:
    def __init__(
        self,
        endpoint: str,
        status_timeout_seconds: float = 1.5,
        generate_timeout_seconds: float = 120.0,
    ) -> None:
        self.endpoint = endpoint.rstrip("/")
        self.status_timeout_seconds = status_timeout_seconds
        self.generate_timeout_seconds = generate_timeout_seconds

    def list_models(self) -> list[OllamaInstalledModel]:
        body = self._request("GET", "/api/tags", timeout_seconds=self.status_timeout_seconds)
        models = body.get("models", [])
        if not isinstance(models, list):
            return []

        installed: list[OllamaInstalledModel] = []
        for item in models:
            if not isinstance(item, dict):
                continue
            name = item.get("name") or item.get("model")
            if not isinstance(name, str) or not name.strip():
                continue
            size = item.get("size")
            installed.append(
                OllamaInstalledModel(
                    name=name,
                    size_bytes=size if isinstance(size, int) else None,
                )
            )
        return installed

    def generate(
        self,
        model: str,
        prompt: str,
        *,
        temperature: float = 0.2,
        num_predict: int = 512,
    ) -> ModelGeneration:
        start = time.perf_counter()
        body = self._request(
            "POST",
            "/api/generate",
            {
                "model": model,
                "prompt": prompt,
                "stream": False,
                "think": THINK_FOR_NORMAL_CHAT,
                "options": {
                    "temperature": temperature,
                    "num_predict": num_predict,
                },
            },
            timeout_seconds=self.generate_timeout_seconds,
        )
        response = body.get("response")
        if not isinstance(response, str):
            raise OllamaProviderError("Ollama returned an invalid generation response.")
        return ModelGeneration(
            model=model,
            response=response.strip(),
            latency_ms=round((time.perf_counter() - start) * 1000),
        )

    def _request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        timeout_seconds: float | None = None,
    ) -> dict[str, Any]:
        data = json.dumps(payload).encode("utf-8") if payload is not None else None
        request = urllib.request.Request(
            f"{self.endpoint}{path}",
            data=data,
            method=method,
            headers={"content-type": "application/json"},
        )

        try:
            with urllib.request.urlopen(
                request,
                timeout=timeout_seconds or self.status_timeout_seconds,
            ) as response:
                content = response.read()
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", errors="replace").strip()
            if error.code == 404:
                model = payload.get("model") if payload else None
                if isinstance(model, str):
                    raise OllamaModelMissingError(model) from error
            raise OllamaProviderError(
                f"Ollama returned HTTP {error.code}: {detail or error.reason}"
            ) from error
        except (TimeoutError, socket.timeout, urllib.error.URLError) as error:
            raise OllamaUnavailableError(
                f"Ollama is not reachable at {self.endpoint}. Start Ollama, then retry."
            ) from error

        if not content:
            return {}

        try:
            parsed = json.loads(content.decode("utf-8"))
        except json.JSONDecodeError as error:
            raise OllamaProviderError("Ollama returned malformed JSON.") from error

        if not isinstance(parsed, dict):
            raise OllamaProviderError("Ollama returned an unexpected response shape.")
        return parsed


class ModelRouter:
    def __init__(self, endpoint: str, store: CoreStore) -> None:
        self.client = OllamaClient(endpoint)
        self.store = store
        self._model_lock = Lock()

    def status(self) -> LocalModelStatusResponse:
        settings = self.store.read_settings()
        selected_chat = settings.selected_chat_model or DEFAULT_CHAT_MODEL
        selected_embedding = settings.selected_embedding_model or DEFAULT_EMBEDDING_MODEL
        installed_models: list[OllamaInstalledModel] = []
        provider_available = False
        message = "Ollama is not running. Start Ollama to enable local chat."

        try:
            installed_models = self.client.list_models()
            provider_available = True
        except OllamaUnavailableError as error:
            message = str(error)
        except OllamaProviderError as error:
            message = str(error)

        installed_names = {model.name for model in installed_models}
        chat_available = selected_chat in installed_names
        embedding_available = selected_embedding in installed_names
        setup_models = self._setup_models(settings.model_profile, installed_models)
        available_models = [
            model for model in setup_models if model.installed
        ] + self._uncatalogued_models(installed_models, setup_models)

        if provider_available and chat_available:
            status = "available"
            message = f"Ollama is ready with {selected_chat}."
        elif provider_available:
            status = "missing"
            message = (
                f"Selected chat model '{selected_chat}' is not installed. "
                f"Run `ollama pull {selected_chat}` or choose an installed model."
            )
        else:
            status = "offline"

        return LocalModelStatusResponse(
            status=status,
            endpoint=self.client.endpoint,
            selected_chat_model=selected_chat,
            selected_embedding_model=selected_embedding,
            recommended_chat_model=DEFAULT_CHAT_MODEL,
            recommended_embedding_model=DEFAULT_EMBEDDING_MODEL,
            chat_model_available=chat_available,
            embedding_model_available=embedding_available,
            available_models=available_models,
            setup_models=setup_models,
            max_parallel_model_jobs=MAX_PARALLEL_MODEL_JOBS,
            think=THINK_FOR_NORMAL_CHAT,
            message=message,
            checked_at=utc_timestamp(),
        )

    def select(self, request: ModelSelectionRequest) -> ModelSelectionResponse:
        settings = self.store.read_settings()
        updates: dict[str, object] = {"updated_at": utc_timestamp()}

        if request.profile is not None:
            updates["model_profile"] = request.profile
        if request.chat_model is not None:
            updates["selected_chat_model"] = normalize_model_name(request.chat_model)
        if request.embedding_model is not None:
            updates["selected_embedding_model"] = normalize_model_name(request.embedding_model)

        next_settings = settings.model_copy(update=updates)
        self.store.write_settings(next_settings)
        return ModelSelectionResponse(settings=next_settings, status=self.status())

    def test_prompt(self, prompt: str) -> ModelGeneration:
        clean_prompt = prompt.strip() or "Reply with exactly: DEYANA_READY"
        model = self._selected_chat_model()
        self._ensure_model_installed(model)
        with self._model_lock:
            return self.client.generate(
                model,
                local_prompt(clean_prompt),
                temperature=0.0,
                num_predict=96,
            )

    def chat(self, content: str) -> ModelGeneration:
        clean_content = content.strip()
        if not clean_content:
            raise ValueError("Chat message cannot be empty.")

        return self.generate_prompt(
            local_prompt(clean_content),
            temperature=0.25,
            num_predict=512,
        )

    def generate_prompt(
        self,
        prompt: str,
        *,
        temperature: float = 0.25,
        num_predict: int = 512,
    ) -> ModelGeneration:
        clean_prompt = prompt.strip()
        if not clean_prompt:
            raise ValueError("Model prompt cannot be empty.")

        model = self._selected_chat_model()
        self._ensure_model_installed(model)
        with self._model_lock:
            return self.client.generate(
                model,
                clean_prompt,
                temperature=temperature,
                num_predict=num_predict,
            )

    def dependency_status(self) -> tuple[str, str]:
        model_status = self.status()
        if model_status.status == "available":
            return "available", model_status.message
        if model_status.status == "missing":
            return "missing", model_status.message
        return "missing", model_status.message

    def _selected_chat_model(self) -> str:
        settings = self.store.read_settings()
        return settings.selected_chat_model or DEFAULT_CHAT_MODEL

    def _ensure_model_installed(self, model: str) -> None:
        installed_names = {installed.name for installed in self.client.list_models()}
        if model not in installed_names:
            raise OllamaModelMissingError(model)

    def _setup_models(
        self,
        profile: ModelProfile,
        installed_models: list[OllamaInstalledModel],
    ) -> list[LocalModelInfo]:
        installed_by_name = {model.name: model for model in installed_models}
        names: list[str] = []
        for name in [*PROFILE_CHAT_MODELS[profile], *PROFILE_EMBEDDING_MODELS[profile]]:
            if name not in names:
                names.append(name)

        return [
            LocalModelInfo(
                name=name,
                role=infer_model_role(name),
                installed=name in installed_by_name,
                recommended=name in {DEFAULT_CHAT_MODEL, DEFAULT_EMBEDDING_MODEL},
                profile=profile,
                size_bytes=installed_by_name[name].size_bytes if name in installed_by_name else None,
                detail=MODEL_DETAILS.get(name, "Local Ollama model."),
            )
            for name in names
        ]

    def _uncatalogued_models(
        self,
        installed_models: list[OllamaInstalledModel],
        setup_models: list[LocalModelInfo],
    ) -> list[LocalModelInfo]:
        known_names = {model.name for model in setup_models}
        return [
            LocalModelInfo(
                name=model.name,
                role=infer_model_role(model.name),
                installed=True,
                recommended=False,
                profile=None,
                size_bytes=model.size_bytes,
                detail="Installed local Ollama model.",
            )
            for model in installed_models
            if model.name not in known_names
        ]


def normalize_model_name(value: str) -> str:
    model = value.strip()
    if not model:
        raise ValueError("Model name cannot be empty.")
    return model


def infer_model_role(name: str) -> str:
    normalized = name.lower()
    if "embed" in normalized or "minilm" in normalized:
        return "embedding"
    if "coder" in normalized or "qwen" in normalized or "llama" in normalized:
        return "chat"
    return "unknown"


def local_prompt(user_content: str) -> str:
    return (
        "You are DE'YANA, a local-first private desktop AI assistant. "
        "Answer using only the local model runtime. Do not suggest cloud AI services. "
        "Be concise, practical, and honest about uncertainty.\n\n"
        f"User: {user_content.strip()}\n"
        "Assistant:"
    )
