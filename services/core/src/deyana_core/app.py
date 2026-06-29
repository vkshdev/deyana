from __future__ import annotations

import asyncio
from contextlib import asynccontextmanager
from typing import AsyncIterator

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from . import __version__
from .api import (
    chat_router,
    connectors_router,
    health_router,
    lifecycle_router,
    memory_router,
    models_router,
    onboarding_router,
    privacy_router,
    release_router,
    settings_router,
    status_router,
    tools_router,
    vault_router,
    voice_router,
    websocket_router,
)
from .identity import PRODUCT_BRAND
from .runtime import RuntimeState


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    runtime: RuntimeState = app.state.runtime
    runtime.mark_running("startup_complete")
    heartbeat_task = asyncio.create_task(publish_heartbeats(runtime))
    await runtime.event_bus.publish(
        runtime.event(
            "backend.lifecycle.changed",
            {"lifecycle": runtime.lifecycle, "reason": "startup_complete"},
        )
    )

    try:
        yield
    finally:
        runtime.mark_stopping("lifespan_shutdown")
        runtime.release_service.mark_clean_shutdown()
        await runtime.event_bus.publish(
            runtime.event(
                "backend.lifecycle.changed",
                {"lifecycle": runtime.lifecycle, "reason": "lifespan_shutdown"},
            )
        )
        heartbeat_task.cancel()
        try:
            await heartbeat_task
        except asyncio.CancelledError:
            pass


async def publish_heartbeats(runtime: RuntimeState) -> None:
    while True:
        await asyncio.sleep(runtime.settings.heartbeat_seconds)
        await runtime.event_bus.publish(
            runtime.event(
                "backend.heartbeat",
                {
                    "lifecycle": runtime.lifecycle,
                    "uptimeSeconds": runtime.uptime_seconds,
                },
            )
        )


def create_app(runtime: RuntimeState | None = None) -> FastAPI:
    from .settings import CoreSettings

    runtime = runtime or RuntimeState(CoreSettings.from_env())
    app = FastAPI(
        title=f"{PRODUCT_BRAND} Core",
        version=__version__,
        lifespan=lifespan,
        docs_url=None,
        redoc_url=None,
    )
    app.state.runtime = runtime

    app.add_middleware(
        CORSMiddleware,
        allow_origins=[
            "http://127.0.0.1:1420",
            "http://localhost:1420",
            "tauri://localhost",
        ],
        allow_methods=["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
        allow_headers=["*"],
    )

    app.include_router(health_router)
    app.include_router(status_router)
    app.include_router(settings_router)
    app.include_router(onboarding_router)
    app.include_router(vault_router)
    app.include_router(memory_router)
    app.include_router(models_router)
    app.include_router(chat_router)
    app.include_router(privacy_router)
    app.include_router(release_router)
    app.include_router(connectors_router)
    app.include_router(tools_router)
    app.include_router(voice_router)
    app.include_router(lifecycle_router)
    app.include_router(websocket_router)
    return app
