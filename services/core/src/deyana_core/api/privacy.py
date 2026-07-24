from __future__ import annotations

from fastapi import APIRouter, Query, Request

from ..models import (
    PrivacyAuditDeleteResponse,
    PrivacyAuditListResponse,
    PrivacyCheckRequest,
    PrivacyCheckResponse,
    PrivacyStatusResponse,
)

router = APIRouter(prefix="/privacy", tags=["privacy"])


@router.get("/status", response_model=PrivacyStatusResponse)
async def privacy_status(request: Request) -> PrivacyStatusResponse:
    return request.app.state.runtime.privacy_firewall.status()


@router.get("/audit", response_model=PrivacyAuditListResponse)
async def privacy_audit(
    request: Request,
    limit: int = Query(default=50, ge=1, le=200),
) -> PrivacyAuditListResponse:
    return request.app.state.runtime.privacy_firewall.list_events(limit=limit)


@router.post("/check", response_model=PrivacyCheckResponse)
async def check_privacy_request(
    request: Request,
    payload: PrivacyCheckRequest,
) -> PrivacyCheckResponse:
    runtime = request.app.state.runtime
    result = runtime.privacy_firewall.check(payload)
    event_type = (
        "privacy.request.blocked" if not result.allowed else "privacy.request.allowed"
    )
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
    return result


@router.post("/test-request", response_model=PrivacyCheckResponse)
async def test_privacy_request(
    request: Request,
    payload: PrivacyCheckRequest,
) -> PrivacyCheckResponse:
    return await check_privacy_request(request, payload)


@router.delete("/audit", response_model=PrivacyAuditDeleteResponse)
async def clear_privacy_audit(request: Request) -> PrivacyAuditDeleteResponse:
    runtime = request.app.state.runtime
    result = runtime.privacy_firewall.clear()
    await runtime.event_bus.publish(
        runtime.event(
            "privacy.audit.deleted",
            result.model_dump(mode="json", by_alias=True),
        )
    )
    return result
