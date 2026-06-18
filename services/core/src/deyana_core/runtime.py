from __future__ import annotations

import os
import platform
import uuid
from datetime import UTC, datetime

from . import __version__
from .agent import ChatAgent
from .chat import ChatStore
from .connectors import ConnectorManager
from .event_bus import EventBus
from .local_models import ModelRouter
from .memory import MemoryStore
from .models import CoreEvent, DependencyStatus, HealthResponse, StatusResponse
from .privacy import PrivacyFirewall
from .runtime_time import utc_timestamp
from .settings import CoreSettings
from .storage import CoreStore


class RuntimeState:
    def __init__(self, settings: CoreSettings) -> None:
        self.settings = settings
        self.version = __version__
        self.boot_id = str(uuid.uuid4())
        self.started_at = datetime.now(UTC)
        self.lifecycle = "running"
        self.event_bus = EventBus()
        self.store = CoreStore(settings.data_dir)
        self.memory_store = MemoryStore(settings.data_dir, self.store)
        self.memory_store.initialize()
        self.chat_store = ChatStore(settings.data_dir)
        self.chat_store.initialize()
        self.model_router = ModelRouter(settings.ollama_endpoint, self.store)
        self.chat_agent = ChatAgent(self.memory_store, self.chat_store, self.model_router)
        self.privacy_firewall = PrivacyFirewall(settings.data_dir, self.store)
        self.privacy_firewall.initialize()
        self.connector_manager = ConnectorManager(settings.data_dir, self.privacy_firewall)
        self.connector_manager.initialize()

    @property
    def uptime_seconds(self) -> float:
        return round((datetime.now(UTC) - self.started_at).total_seconds(), 3)

    def mark_running(self, _reason: str) -> None:
        self.lifecycle = "running"

    def mark_stopping(self, _reason: str) -> None:
        self.lifecycle = "stopping"

    def health(self) -> HealthResponse:
        return HealthResponse(
            version=self.version,
            lifecycle=self.lifecycle,
            uptime_seconds=self.uptime_seconds,
            timestamp=self.timestamp(),
        )

    def status(self) -> StatusResponse:
        ollama_status, ollama_detail = self.model_router.dependency_status()
        return StatusResponse(
            version=self.version,
            lifecycle=self.lifecycle,
            boot_id=self.boot_id,
            pid=os.getpid(),
            uptime_seconds=self.uptime_seconds,
            host=self.settings.host,
            port=self.settings.port,
            dependencies=[
                DependencyStatus(
                    name="python",
                    status="available",
                    detail=platform.python_version(),
                ),
                DependencyStatus(
                    name="sqlite",
                    status="available",
                    detail="SQLite memory and chat storage initialized.",
                ),
                DependencyStatus(
                    name="ollama",
                    status=ollama_status,
                    detail=ollama_detail,
                ),
                DependencyStatus(
                    name="privacy_firewall",
                    status="available",
                    detail="Local-only external request policy and audit log initialized.",
                ),
                DependencyStatus(
                    name="connector_store",
                    status="available",
                    detail="Connector registry, encrypted token storage, and sync run logs initialized.",
                ),
            ],
            feature_flags={
                "websocketEvents": True,
                "localLogging": True,
                "settings": True,
                "onboarding": True,
                "vaultSetup": True,
                "memory": True,
                "models": True,
                "chat": True,
                "memoryRetrieval": True,
                "privacyFirewall": True,
                "privacyAudit": True,
                "connectors": True,
                "connectorScheduler": True,
                "encryptedTokenStorage": True,
                "voice": False,
            },
            timestamp=self.timestamp(),
        )

    def event(self, event_type: str, payload: dict[str, object]) -> CoreEvent:
        return CoreEvent(
            id=str(uuid.uuid4()),
            type=event_type,
            timestamp=self.timestamp(),
            payload=payload,
        )

    @staticmethod
    def timestamp() -> str:
        return utc_timestamp()
