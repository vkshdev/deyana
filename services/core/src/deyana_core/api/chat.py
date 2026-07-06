from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Query, Request

from ..local_models import (
    OllamaModelMissingError,
    OllamaProviderError,
    OllamaUnavailableError,
)
from ..models import (
    ChatHistoryDeleteResponse,
    ChatHistoryResponse,
    ChatMessageRequest,
    ChatMessageResponse,
)

router = APIRouter(prefix="/chat", tags=["chat"])


@router.get("/history", response_model=ChatHistoryResponse)
async def chat_history(
    request: Request,
    limit: int = Query(default=50, ge=1, le=200),
) -> ChatHistoryResponse:
    return ChatHistoryResponse(messages=request.app.state.runtime.chat_store.history(limit=limit))


@router.post("/message", response_model=ChatMessageResponse)
async def send_chat_message(
    request: Request,
    payload: ChatMessageRequest,
) -> ChatMessageResponse:
    runtime = request.app.state.runtime
    content = payload.content.strip()
    if not content:
        raise HTTPException(status_code=400, detail="Chat message cannot be empty.")

    try:
        response = await asyncio.to_thread(
            runtime.chat_agent.answer,
            content,
            use_memory=payload.use_memory,
            allow_web=payload.allow_web,
        )
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error
    except OllamaUnavailableError as error:
        raise HTTPException(status_code=503, detail=str(error)) from error
    except OllamaModelMissingError as error:
        raise HTTPException(status_code=409, detail=str(error)) from error
    except OllamaProviderError as error:
        raise HTTPException(status_code=502, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "chat.message.created",
            response.model_dump(mode="json", by_alias=True),
        )
    )
    return response


@router.delete("/history", response_model=ChatHistoryDeleteResponse)
async def clear_chat_history(request: Request) -> ChatHistoryDeleteResponse:
    runtime = request.app.state.runtime
    deleted = runtime.chat_store.clear()
    response = ChatHistoryDeleteResponse(deleted=deleted)
    await runtime.event_bus.publish(
        runtime.event(
            "chat.history.deleted",
            response.model_dump(mode="json", by_alias=True),
        )
    )
    return response
