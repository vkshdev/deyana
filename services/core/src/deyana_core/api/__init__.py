from .health import router as health_router
from .lifecycle import router as lifecycle_router
from .memory import router as memory_router
from .onboarding import router as onboarding_router
from .settings import router as settings_router
from .status import router as status_router
from .vault import router as vault_router
from .websocket import router as websocket_router

__all__ = [
    "health_router",
    "lifecycle_router",
    "memory_router",
    "onboarding_router",
    "settings_router",
    "status_router",
    "vault_router",
    "websocket_router",
]
