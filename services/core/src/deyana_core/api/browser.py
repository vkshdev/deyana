from __future__ import annotations

from fastapi import APIRouter, HTTPException, Query, Request

from ..browser.models import (
    BrowserActionConfirmRequest,
    BrowserActionPlanCreateRequest,
    BrowserActionPlanListResponse,
    BrowserActionPlanResponse,
    BrowserEmergencyStopResponse,
    BrowserAuditListResponse,
    BrowserContextReadRequest,
    BrowserContextReadResponse,
    BrowserContextSummaryRequest,
    BrowserContextSummaryResponse,
    BrowserClearFieldRequest,
    BrowserClickActionRequest,
    BrowserClickActionResponse,
    BrowserContactTonePreference,
    BrowserContactTonePreferenceRequest,
    BrowserDisconnectResponse,
    BrowserDraftReplyRequest,
    BrowserDraftReplyResponse,
    BrowserFillFieldRequest,
    BrowserFillFieldResponse,
    BrowserMoodHint,
    BrowserMoodInferRequest,
    BrowserOpenTabRequest,
    BrowserOpenTabResponse,
    BrowserPersonalityPreviewRequest,
    BrowserPersonalityPreviewResponse,
    BrowserPersonalityProfile,
    BrowserPersonalityProfilePatch,
    BrowserPersonalitySettingsResponse,
    BrowserPermissionListResponse,
    BrowserPermissionRequest,
    BrowserPermissionResponse,
    BrowserSearchRequest,
    BrowserSearchResponse,
    BrowserSessionListResponse,
    BrowserStatusResponse,
    BrowserVoiceCommandRequest,
    BrowserVoiceCommandResponse,
    WhatsAppBusyModeEvaluationRequest,
    WhatsAppBusyModeEvaluationResponse,
    WhatsAppBusyModePolicy,
    WhatsAppBusyModePolicyPatch,
    WhatsAppBusyModePolicyResponse,
    WhatsAppBusyModeSendRequest,
    WhatsAppBusyModeSendResponse,
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


@router.post("/messages/draft-reply", response_model=BrowserDraftReplyResponse)
async def draft_browser_reply(
    request: Request,
    payload: BrowserDraftReplyRequest,
) -> BrowserDraftReplyResponse:
    return await request.app.state.runtime.browser_service.draft_reply(payload)


@router.post("/fields/fill", response_model=BrowserFillFieldResponse)
async def fill_browser_field(
    request: Request,
    payload: BrowserFillFieldRequest,
) -> BrowserFillFieldResponse:
    return await request.app.state.runtime.browser_service.fill_field(payload)


@router.post("/fields/clear", response_model=BrowserFillFieldResponse)
async def clear_browser_field(
    request: Request,
    payload: BrowserClearFieldRequest,
) -> BrowserFillFieldResponse:
    return await request.app.state.runtime.browser_service.clear_field(payload)


@router.post("/actions/click", response_model=BrowserClickActionResponse)
async def click_browser_action(
    request: Request,
    payload: BrowserClickActionRequest,
) -> BrowserClickActionResponse:
    return await request.app.state.runtime.browser_service.click_action(payload)


@router.get("/audit", response_model=BrowserAuditListResponse)
async def browser_audit(
    request: Request,
    limit: int = Query(default=50, ge=1, le=200),
) -> BrowserAuditListResponse:
    return request.app.state.runtime.browser_service.list_audit(limit=limit)


@router.get("/actions/plans", response_model=BrowserActionPlanListResponse)
async def browser_action_plans(
    request: Request,
    limit: int = Query(default=20, ge=1, le=100),
) -> BrowserActionPlanListResponse:
    return request.app.state.runtime.browser_service.list_action_plans(limit=limit)


@router.post("/actions/plans", response_model=BrowserActionPlanResponse)
async def create_browser_action_plan(
    request: Request,
    payload: BrowserActionPlanCreateRequest,
) -> BrowserActionPlanResponse:
    return request.app.state.runtime.browser_service.create_action_plan(payload)


@router.post("/actions/confirm", response_model=BrowserActionPlanResponse)
async def confirm_browser_action_plan(
    request: Request,
    payload: BrowserActionConfirmRequest,
) -> BrowserActionPlanResponse:
    return request.app.state.runtime.browser_service.confirm_action_plan(payload)


@router.post("/actions/plans/{plan_id}/execute", response_model=BrowserActionPlanResponse)
async def execute_browser_action_plan(
    request: Request,
    plan_id: str,
) -> BrowserActionPlanResponse:
    return await request.app.state.runtime.browser_service.execute_action_plan(plan_id)


@router.post("/actions/plans/{plan_id}/cancel", response_model=BrowserActionPlanResponse)
async def cancel_browser_action_plan(
    request: Request,
    plan_id: str,
) -> BrowserActionPlanResponse:
    return request.app.state.runtime.browser_service.cancel_action_plan(plan_id)


@router.post("/actions/emergency-stop", response_model=BrowserEmergencyStopResponse)
async def browser_emergency_stop(request: Request) -> BrowserEmergencyStopResponse:
    return request.app.state.runtime.browser_service.emergency_stop()


@router.get("/whatsapp/busy-mode", response_model=WhatsAppBusyModePolicy)
async def get_whatsapp_busy_mode(request: Request) -> WhatsAppBusyModePolicy:
    return request.app.state.runtime.browser_service.get_whatsapp_busy_policy()


@router.patch("/whatsapp/busy-mode", response_model=WhatsAppBusyModePolicyResponse)
async def patch_whatsapp_busy_mode(
    request: Request,
    payload: WhatsAppBusyModePolicyPatch,
) -> WhatsAppBusyModePolicyResponse:
    try:
        return await request.app.state.runtime.browser_service.update_whatsapp_busy_policy(payload)
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error


@router.post("/whatsapp/busy-mode/evaluate", response_model=WhatsAppBusyModeEvaluationResponse)
async def evaluate_whatsapp_busy_mode(
    request: Request,
    payload: WhatsAppBusyModeEvaluationRequest,
) -> WhatsAppBusyModeEvaluationResponse:
    return request.app.state.runtime.browser_service.evaluate_whatsapp_busy_mode(payload)


@router.post("/whatsapp/busy-mode/send", response_model=WhatsAppBusyModeSendResponse)
async def send_whatsapp_busy_reply(
    request: Request,
    payload: WhatsAppBusyModeSendRequest,
) -> WhatsAppBusyModeSendResponse:
    return await request.app.state.runtime.browser_service.send_whatsapp_busy_reply(payload)


@router.get("/personality", response_model=BrowserPersonalitySettingsResponse)
async def get_browser_personality(request: Request) -> BrowserPersonalitySettingsResponse:
    return request.app.state.runtime.browser_service.get_personality_settings()


@router.patch("/personality/profile", response_model=BrowserPersonalityProfile)
async def patch_browser_personality_profile(
    request: Request,
    payload: BrowserPersonalityProfilePatch,
) -> BrowserPersonalityProfile:
    try:
        return request.app.state.runtime.browser_service.update_personality_profile(payload)
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error


@router.post("/personality/contact-tones", response_model=BrowserContactTonePreference)
async def save_browser_contact_tone(
    request: Request,
    payload: BrowserContactTonePreferenceRequest,
) -> BrowserContactTonePreference:
    return request.app.state.runtime.browser_service.save_contact_tone(payload)


@router.delete("/personality/contact-tones", response_model=dict[str, bool])
async def delete_browser_contact_tone(
    request: Request,
    adapter_id: str = Query(min_length=1, max_length=80, alias="adapterId"),
    contact_label: str = Query(min_length=1, max_length=160, alias="contactLabel"),
) -> dict[str, bool]:
    deleted = request.app.state.runtime.browser_service.delete_contact_tone(adapter_id, contact_label)
    return {"deleted": deleted}


@router.post("/personality/mood/infer", response_model=BrowserMoodHint)
async def infer_browser_mood(
    request: Request,
    payload: BrowserMoodInferRequest,
) -> BrowserMoodHint:
    return request.app.state.runtime.browser_service.infer_mood(payload)


@router.post("/personality/preview", response_model=BrowserPersonalityPreviewResponse)
async def preview_browser_personality(
    request: Request,
    payload: BrowserPersonalityPreviewRequest,
) -> BrowserPersonalityPreviewResponse:
    return request.app.state.runtime.browser_service.preview_personality(payload)


@router.post("/voice/command", response_model=BrowserVoiceCommandResponse)
async def route_browser_voice_command(
    request: Request,
    payload: BrowserVoiceCommandRequest,
) -> BrowserVoiceCommandResponse:
    return await request.app.state.runtime.browser_service.route_voice_command(payload)
