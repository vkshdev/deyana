from .chat import router as chat_router
from .connectors import router as connectors_router
from .health import router as health_router
from .lifecycle import router as lifecycle_router
from .memory import router as memory_router
from .models import router as models_router
from .onboarding import router as onboarding_router
from .privacy import router as privacy_router
from .release import router as release_router
from .settings import router as settings_router
from .status import router as status_router
from .tools import router as tools_router
from .vault import router as vault_router
from .voice import router as voice_router
from .websocket import router as websocket_router

__all__ = [
    "chat_router",
    "connectors_router",
    "health_router",
    "lifecycle_router",
    "memory_router",
    "models_router",
    "onboarding_router",
    "privacy_router",
    "release_router",
    "settings_router",
    "status_router",
    "tools_router",
    "vault_router",
    "voice_router",
    "websocket_router",
]
