from __future__ import annotations

from fastapi import APIRouter, HTTPException, Query, Request

from ..models import (
    MemoryCreateRequest,
    MemoryDeleteResponse,
    MemoryExportResponse,
    MemoryItem,
    MemoryListResponse,
    MemoryReindexResponse,
    MemoryUpdateRequest,
)

router = APIRouter(prefix="/memory", tags=["memory"])


@router.get("", response_model=MemoryListResponse)
async def list_memory(
    request: Request,
    query: str | None = Query(default=None),
    limit: int = Query(default=20, ge=1, le=100),
) -> MemoryListResponse:
    return request.app.state.runtime.memory_store.list(query=query, limit=limit)


@router.post("", response_model=MemoryItem)
async def create_memory(request: Request, payload: MemoryCreateRequest) -> MemoryItem:
    runtime = request.app.state.runtime
    try:
        item = runtime.memory_store.create(payload)
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "memory.item.created",
            {"item": item.model_dump(mode="json", by_alias=True)},
        )
    )
    return item


@router.get("/export", response_model=MemoryExportResponse)
async def export_memory(request: Request) -> MemoryExportResponse:
    return request.app.state.runtime.memory_store.export()


@router.post("/reindex", response_model=MemoryReindexResponse)
async def reindex_memory(request: Request) -> MemoryReindexResponse:
    runtime = request.app.state.runtime
    result = runtime.memory_store.reindex()
    await runtime.event_bus.publish(
        runtime.event(
            "memory.reindexed",
            result.model_dump(mode="json", by_alias=True),
        )
    )
    return result


@router.get("/{memory_id}", response_model=MemoryItem)
async def get_memory(request: Request, memory_id: str) -> MemoryItem:
    try:
        return request.app.state.runtime.memory_store.get(memory_id)
    except KeyError as error:
        raise HTTPException(status_code=404, detail="Memory item not found.") from error


@router.patch("/{memory_id}", response_model=MemoryItem)
async def update_memory(
    request: Request, memory_id: str, payload: MemoryUpdateRequest
) -> MemoryItem:
    runtime = request.app.state.runtime
    try:
        item = runtime.memory_store.update(memory_id, payload)
    except KeyError as error:
        raise HTTPException(status_code=404, detail="Memory item not found.") from error

    await runtime.event_bus.publish(
        runtime.event(
            "memory.item.updated",
            {"item": item.model_dump(mode="json", by_alias=True)},
        )
    )
    return item


@router.delete("/{memory_id}", response_model=MemoryDeleteResponse)
async def delete_memory(request: Request, memory_id: str) -> MemoryDeleteResponse:
    runtime = request.app.state.runtime
    try:
        deleted = runtime.memory_store.delete(memory_id)
    except KeyError as error:
        raise HTTPException(status_code=404, detail="Memory item not found.") from error

    await runtime.event_bus.publish(
        runtime.event("memory.item.deleted", {"id": memory_id, "deleted": deleted})
    )
    return MemoryDeleteResponse(deleted=deleted, id=memory_id)
