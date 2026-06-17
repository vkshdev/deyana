from __future__ import annotations

from fastapi import APIRouter, Request

from ..models import AppSettings, SettingsPatch

router = APIRouter(tags=["settings"])


@router.get("/settings", response_model=AppSettings)
async def get_settings(request: Request) -> AppSettings:
    return request.app.state.runtime.store.read_settings()


@router.patch("/settings", response_model=AppSettings)
async def patch_settings(request: Request, patch: SettingsPatch) -> AppSettings:
    runtime = request.app.state.runtime
    settings = runtime.store.patch_settings(patch)
    await runtime.event_bus.publish(
        runtime.event(
            "settings.changed",
            {"settings": settings.model_dump(mode="json", by_alias=True)},
        )
    )
    return settings


@router.post("/settings/reset", response_model=AppSettings)
async def reset_settings(request: Request) -> AppSettings:
    runtime = request.app.state.runtime
    settings = runtime.store.reset_settings()
    await runtime.event_bus.publish(
        runtime.event(
            "settings.changed",
            {"settings": settings.model_dump(mode="json", by_alias=True)},
        )
    )
    return settings
