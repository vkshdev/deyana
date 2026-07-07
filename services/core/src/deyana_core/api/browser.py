from __future__ import annotations

from fastapi import APIRouter, HTTPException, Query, Request

from ..browser.models import (
    BrowserAuditListResponse,
    BrowserContextReadRequest,
    BrowserContextReadResponse,
    BrowserContextSummaryRequest,
    BrowserContextSummaryResponse,
    BrowserDisconnectResponse,
    BrowserOpenTabRequest,
    BrowserOpenTabResponse,
    BrowserPermissionListResponse,
    BrowserPermissionRequest,
    BrowserPermissionResponse,
    BrowserSearchRequest,
    BrowserSearchResponse,
    BrowserSessionListResponse,
    BrowserStatusResponse,
)
from ..privacy import PrivacyPolicyError
from ..tools import ToolExecutionError

router = APIRouter(prefix="/browser", tags=["browser"])


@router.get("/status", response_model=BrowserStatusResponse)
async def browser_status(request: Request) -> BrowserStatusResponse:
    return request.app.state.runtime.browser_service.status()


@router.get("/sessions", response_model=BrowserSessionListResponse)
async def browser_sessions(request: Request) -> BrowserSessionListResponse:
    return request.app.state.runtime.browser_service.list_sessions()


@router.delete("/sessions/{page_session_id}", response_model=BrowserDisconnectResponse)
async def disconnect_browser_session(
    request: Request,
    page_session_id: str,
) -> BrowserDisconnectResponse:
    return await request.app.state.runtime.browser_service.disconnect_session(page_session_id)


@router.get("/permissions", response_model=BrowserPermissionListResponse)
async def browser_permissions(request: Request) -> BrowserPermissionListResponse:
    return request.app.state.runtime.browser_service.list_permissions()


@router.post("/permissions/request", response_model=BrowserPermissionResponse)
async def request_browser_permission(
    request: Request,
    payload: BrowserPermissionRequest,
) -> BrowserPermissionResponse:
    return await request.app.state.runtime.browser_service.request_permission(payload)


@router.delete("/permissions", response_model=BrowserPermissionResponse)
async def revoke_browser_permission(
    request: Request,
    origin: str = Query(min_length=1, max_length=2048),
) -> BrowserPermissionResponse:
    return await request.app.state.runtime.browser_service.revoke_permission(origin)


@router.post("/context/read", response_model=BrowserContextReadResponse)
async def read_browser_context(
    request: Request,
    payload: BrowserContextReadRequest,
) -> BrowserContextReadResponse:
    return await request.app.state.runtime.browser_service.request_context(payload)


@router.post("/context/summarize", response_model=BrowserContextSummaryResponse)
async def summarize_browser_context(
    request: Request,
    payload: BrowserContextSummaryRequest,
) -> BrowserContextSummaryResponse:
    return await request.app.state.runtime.browser_service.summarize_context(payload)


@router.post("/search", response_model=BrowserSearchResponse)
async def browser_search(
    request: Request,
    payload: BrowserSearchRequest,
) -> BrowserSearchResponse:
    try:
        return await request.app.state.runtime.browser_service.search(payload)
    except PrivacyPolicyError as error:
        raise HTTPException(status_code=403, detail=error.response.reason) from error
    except ToolExecutionError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error


@router.post("/tabs/open", response_model=BrowserOpenTabResponse)
async def open_browser_tab(
    request: Request,
    payload: BrowserOpenTabRequest,
) -> BrowserOpenTabResponse:
    try:
        return await request.app.state.runtime.browser_service.open_tab(payload)
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error


@router.get("/audit", response_model=BrowserAuditListResponse)
async def browser_audit(
    request: Request,
    limit: int = Query(default=50, ge=1, le=200),
) -> BrowserAuditListResponse:
    return request.app.state.runtime.browser_service.list_audit(limit=limit)
