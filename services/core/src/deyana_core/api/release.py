from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Query, Request

from ..models import (
    ConnectorHealthResponse,
    DeleteLocalDataRequest,
    DeleteLocalDataResponse,
    CrashRecoveryResponse,
    PerformanceProfileResponse,
    ReleaseLogListResponse,
    ReleaseLogReadResponse,
    ReleasePrivacyExportResponse,
    ReleaseReadinessResponse,
    ReleaseUpdatePlanResponse,
)
from ..release import ReleaseSafetyError

router = APIRouter(prefix="/release", tags=["release"])


@router.get("/readiness", response_model=ReleaseReadinessResponse)
async def release_readiness(request: Request) -> ReleaseReadinessResponse:
    runtime = request.app.state.runtime
    return await asyncio.to_thread(runtime.release_service.readiness, runtime.version)


@router.get("/update-plan", response_model=ReleaseUpdatePlanResponse)
async def release_update_plan(request: Request) -> ReleaseUpdatePlanResponse:
    runtime = request.app.state.runtime
    return runtime.release_service.update_plan(runtime.version)


@router.get("/logs", response_model=ReleaseLogListResponse)
async def release_logs(request: Request) -> ReleaseLogListResponse:
    return await asyncio.to_thread(request.app.state.runtime.release_service.list_logs)


@router.get("/logs/read", response_model=ReleaseLogReadResponse)
async def read_release_log(
    request: Request,
    path: str = Query(..., min_length=1),
    max_characters: int = Query(default=20000, ge=1000, le=100000, alias="maxCharacters"),
) -> ReleaseLogReadResponse:
    try:
        return await asyncio.to_thread(
            request.app.state.runtime.release_service.read_log,
            path,
            max_characters,
        )
    except FileNotFoundError as error:
        raise HTTPException(status_code=404, detail="Log file not found.") from error
    except ReleaseSafetyError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error


@router.get("/privacy-export", response_model=ReleasePrivacyExportResponse)
async def release_privacy_export(request: Request) -> ReleasePrivacyExportResponse:
    return await asyncio.to_thread(request.app.state.runtime.release_service.privacy_export)


@router.post("/delete-local-data", response_model=DeleteLocalDataResponse)
async def delete_local_data(request: Request, payload: DeleteLocalDataRequest) -> DeleteLocalDataResponse:
    runtime = request.app.state.runtime
    try:
        response = await asyncio.to_thread(
            runtime.release_service.delete_local_data,
            confirmation_phrase=payload.confirmation_phrase,
            include_vault=payload.include_vault,
        )
    except ReleaseSafetyError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "release.local_data.deleted",
            response.model_dump(mode="json", by_alias=True),
        )
    )
    return response


@router.get("/connector-health", response_model=ConnectorHealthResponse)
async def connector_health(request: Request) -> ConnectorHealthResponse:
    return await asyncio.to_thread(request.app.state.runtime.release_service.connector_health)


@router.get("/performance", response_model=PerformanceProfileResponse)
async def performance_profile(request: Request) -> PerformanceProfileResponse:
    runtime = request.app.state.runtime
    return await asyncio.to_thread(runtime.release_service.performance_profile, runtime.uptime_seconds)


@router.get("/crash-recovery", response_model=CrashRecoveryResponse)
async def crash_recovery(request: Request) -> CrashRecoveryResponse:
    return request.app.state.runtime.release_service.crash_recovery()
