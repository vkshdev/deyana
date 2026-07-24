from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Request

from ..local_models import (
    OllamaModelMissingError,
    OllamaProviderError,
    OllamaUnavailableError,
)
from ..models import (
    LocalModelStatusResponse,
    ModelSelectionRequest,
    ModelSelectionResponse,
    ModelTestRequest,
    ModelTestResponse,
)

router = APIRouter(tags=["models"])


@router.get("/models/status", response_model=LocalModelStatusResponse)
async def model_status(request: Request) -> LocalModelStatusResponse:
    return request.app.state.runtime.model_router.status()


@router.patch("/models/selection", response_model=ModelSelectionResponse)
async def select_model(
    request: Request, payload: ModelSelectionRequest
) -> ModelSelectionResponse:
    runtime = request.app.state.runtime
    try:
        result = runtime.model_router.select(payload)
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "models.status.changed",
            result.status.model_dump(mode="json", by_alias=True),
        )
    )
    return result


@router.post("/model/test", response_model=ModelTestResponse)
async def test_model(request: Request, payload: ModelTestRequest) -> ModelTestResponse:
    runtime = request.app.state.runtime
    try:
        generation = await asyncio.to_thread(
            runtime.model_router.test_prompt, payload.prompt
        )
    except OllamaUnavailableError as error:
        raise HTTPException(status_code=503, detail=str(error)) from error
    except OllamaModelMissingError as error:
        raise HTTPException(status_code=409, detail=str(error)) from error
    except OllamaProviderError as error:
        raise HTTPException(status_code=502, detail=str(error)) from error

    response = ModelTestResponse(
        ok=True,
        model=generation.model,
        response=generation.response,
        latency_ms=generation.latency_ms,
        detail="Local test prompt completed through Ollama.",
    )
    await runtime.event_bus.publish(
        runtime.event(
            "models.test.completed",
            response.model_dump(mode="json", by_alias=True),
        )
    )
    return response
