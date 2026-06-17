from __future__ import annotations

from fastapi import APIRouter, HTTPException, Request

from ..models import OnboardingCompleteRequest, OnboardingCompleteResponse, OnboardingState

router = APIRouter(tags=["onboarding"])


@router.get("/onboarding/state", response_model=OnboardingState)
async def get_onboarding_state(request: Request) -> OnboardingState:
    return request.app.state.runtime.store.read_onboarding()


@router.post("/onboarding/complete", response_model=OnboardingCompleteResponse)
async def complete_onboarding(
    request: Request, payload: OnboardingCompleteRequest
) -> OnboardingCompleteResponse:
    runtime = request.app.state.runtime
    try:
        state, settings = runtime.store.complete_onboarding(
            privacy_mode=payload.privacy_mode,
            model_profile=payload.model_profile,
            vault_path=payload.vault_path,
        )
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "onboarding.state.changed",
            {
                "state": state.model_dump(mode="json", by_alias=True),
                "settings": settings.model_dump(mode="json", by_alias=True),
            },
        )
    )
    return OnboardingCompleteResponse(state=state, settings=settings)
