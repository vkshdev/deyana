from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Query, Request

from ..connectors import ConnectorError, ConnectorNotFoundError, ConnectorStateError
from ..models import (
    ConnectorDisconnectResponse,
    ConnectorItem,
    ConnectorListResponse,
    ConnectorOAuthCompleteRequest,
    ConnectorOAuthStartRequest,
    ConnectorOAuthStartResponse,
    ConnectorSettingsPatch,
    ConnectorSyncRequest,
    ConnectorSyncResponse,
    ConnectorSyncRunsResponse,
    PrivacyCheckResponse,
)
from ..privacy import PrivacyPolicyError

router = APIRouter(prefix="/connectors", tags=["connectors"])


@router.get("", response_model=ConnectorListResponse)
async def list_connectors(request: Request) -> ConnectorListResponse:
    return await asyncio.to_thread(request.app.state.runtime.connector_manager.list_connectors)


@router.get("/sync-runs", response_model=ConnectorSyncRunsResponse)
async def list_connector_sync_runs(
    request: Request,
    limit: int = Query(default=20, ge=1, le=100),
) -> ConnectorSyncRunsResponse:
    return await asyncio.to_thread(request.app.state.runtime.connector_manager.list_sync_runs, limit=limit)


@router.get("/{connector_id}", response_model=ConnectorItem)
async def get_connector(request: Request, connector_id: str) -> ConnectorItem:
    try:
        return await asyncio.to_thread(request.app.state.runtime.connector_manager.get_connector, connector_id)
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error


@router.patch("/{connector_id}/settings", response_model=ConnectorItem)
async def update_connector_settings(
    request: Request,
    connector_id: str,
    payload: ConnectorSettingsPatch,
) -> ConnectorItem:
    runtime = request.app.state.runtime
    try:
        connector = await asyncio.to_thread(
            runtime.connector_manager.update_settings,
            connector_id,
            enabled=payload.enabled,
            sync_interval_minutes=payload.sync_interval_minutes,
        )
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error

    await publish_connector_event(runtime, "connector.status.changed", connector)
    return connector


@router.post("/{connector_id}/oauth/start", response_model=ConnectorOAuthStartResponse)
async def start_connector_oauth(
    request: Request,
    connector_id: str,
    payload: ConnectorOAuthStartRequest,
) -> ConnectorOAuthStartResponse:
    runtime = request.app.state.runtime
    try:
        response = await asyncio.to_thread(
            runtime.connector_manager.start_oauth,
            connector_id,
            payload.redirect_uri,
        )
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "connector.oauth.started",
            response.model_dump(mode="json", by_alias=True),
        )
    )
    return response


@router.post("/{connector_id}/oauth/complete", response_model=ConnectorItem)
async def complete_connector_oauth(
    request: Request,
    connector_id: str,
    payload: ConnectorOAuthCompleteRequest,
) -> ConnectorItem:
    runtime = request.app.state.runtime
    try:
        connector, privacy_result = await asyncio.to_thread(
            runtime.connector_manager.complete_oauth,
            connector_id,
            state=payload.state,
            code=payload.code,
            user_approved=payload.user_approved,
        )
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error
    except ConnectorStateError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error
    except ConnectorError as error:
        raise HTTPException(status_code=502, detail=str(error)) from error
    except PrivacyPolicyError as error:
        await publish_privacy_event(runtime, error.response)
        raise HTTPException(status_code=403, detail=error.response.reason) from error

    await publish_privacy_event(runtime, privacy_result)
    await publish_connector_event(runtime, "connector.oauth.completed", connector)
    await publish_connector_event(runtime, "connector.status.changed", connector)
    return connector


@router.post("/{connector_id}/disconnect", response_model=ConnectorDisconnectResponse)
async def disconnect_connector(request: Request, connector_id: str) -> ConnectorDisconnectResponse:
    runtime = request.app.state.runtime
    try:
        connector, token_deleted = await asyncio.to_thread(
            runtime.connector_manager.disconnect,
            connector_id,
        )
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error

    response = ConnectorDisconnectResponse(connector=connector, token_deleted=token_deleted)
    await publish_connector_event(runtime, "connector.status.changed", connector)
    return response


@router.post("/{connector_id}/sync", response_model=ConnectorSyncResponse)
async def sync_connector(
    request: Request,
    connector_id: str,
    payload: ConnectorSyncRequest,
) -> ConnectorSyncResponse:
    runtime = request.app.state.runtime
    try:
        started = await asyncio.to_thread(
            runtime.connector_manager.start_sync,
            connector_id,
            reason=payload.reason,
        )
    except ConnectorNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error
    except ConnectorStateError as error:
        await publish_connector_error_snapshot(runtime, connector_id)
        raise HTTPException(status_code=409, detail=str(error)) from error

    await publish_connector_sync_event(runtime, "connector.sync.started", started)

    if started.run.status == "skipped":
        await publish_connector_sync_event(runtime, "connector.sync.skipped", started)
        return started

    try:
        response, privacy_result = await asyncio.to_thread(
            runtime.connector_manager.finish_sync,
            connector_id,
            started.run.id,
        )
    except PrivacyPolicyError as error:
        await publish_privacy_event(runtime, error.response)
        await publish_connector_error_snapshot(runtime, connector_id)
        raise HTTPException(status_code=403, detail=error.response.reason) from error
    except ConnectorError as error:
        await publish_connector_error_snapshot(runtime, connector_id)
        raise HTTPException(status_code=409, detail=str(error)) from error

    if privacy_result:
        await publish_privacy_event(runtime, privacy_result)
    event_type = "connector.sync.completed" if response.run.status == "completed" else "connector.sync.failed"
    await publish_connector_sync_event(runtime, event_type, response)
    await publish_connector_event(runtime, "connector.status.changed", response.connector)
    return response


async def publish_connector_event(runtime, event_type: str, connector: ConnectorItem) -> None:
    await runtime.event_bus.publish(
        runtime.event(
            event_type,
            {"connector": connector.model_dump(mode="json", by_alias=True)},
        )
    )


async def publish_connector_sync_event(runtime, event_type: str, response: ConnectorSyncResponse) -> None:
    await runtime.event_bus.publish(
        runtime.event(
            event_type,
            response.model_dump(mode="json", by_alias=True),
        )
    )


async def publish_connector_error_snapshot(runtime, connector_id: str) -> None:
    try:
        connector = await asyncio.to_thread(runtime.connector_manager.get_connector, connector_id)
    except ConnectorError:
        return
    await publish_connector_event(runtime, "connector.status.changed", connector)


async def publish_privacy_event(runtime, result: PrivacyCheckResponse) -> None:
    event_type = "privacy.request.blocked" if not result.allowed else "privacy.request.allowed"
    await runtime.event_bus.publish(
        runtime.event(
            event_type,
            {
                "reason": result.reason,
                "destination": result.destination,
                "destinationCategory": result.destination_category,
                "dataType": result.data_category,
                "safeAlternative": result.safe_alternative,
                "auditEvent": result.audit_event.model_dump(mode="json", by_alias=True),
            },
        )
    )
