from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Request

from ..models import (
    CodeTaskRequest,
    DayPlannerRequest,
    FileReadRequest,
    GitReadRequest,
    ToolListResponse,
    ToolRunResponse,
    WebFetchRequest,
    WebSearchRequest,
)
from ..privacy import PrivacyPolicyError
from ..tools import ToolExecutionError

router = APIRouter(prefix="/tools", tags=["tools"])


@router.get("", response_model=ToolListResponse)
async def list_tools(request: Request) -> ToolListResponse:
    return request.app.state.runtime.tool_service.list_tools()


@router.post("/web-search", response_model=ToolRunResponse)
async def web_search(request: Request, payload: WebSearchRequest) -> ToolRunResponse:
    return await run_tool(request, "web_search", payload)


@router.post("/fetch-page", response_model=ToolRunResponse)
async def fetch_page(request: Request, payload: WebFetchRequest) -> ToolRunResponse:
    return await run_tool(request, "fetch_page", payload)


@router.post("/read-file", response_model=ToolRunResponse)
async def read_file(request: Request, payload: FileReadRequest) -> ToolRunResponse:
    return await run_tool(request, "read_file", payload)


@router.post("/git/status", response_model=ToolRunResponse)
async def git_status(request: Request, payload: GitReadRequest) -> ToolRunResponse:
    return await run_tool(request, "git_status", payload)


@router.post("/git/diff", response_model=ToolRunResponse)
async def git_diff(request: Request, payload: GitReadRequest) -> ToolRunResponse:
    return await run_tool(request, "git_diff", payload)


@router.post("/git/commit-message", response_model=ToolRunResponse)
async def commit_message(request: Request, payload: GitReadRequest) -> ToolRunResponse:
    return await run_tool(request, "commit_message", payload)


@router.post("/code/task", response_model=ToolRunResponse)
async def code_task(request: Request, payload: CodeTaskRequest) -> ToolRunResponse:
    return await run_tool(request, "code_task", payload)


@router.post("/day-planner", response_model=ToolRunResponse)
async def day_planner(request: Request, payload: DayPlannerRequest) -> ToolRunResponse:
    return await run_tool(request, "day_planner", payload)


async def run_tool(request: Request, method_name: str, payload) -> ToolRunResponse:
    runtime = request.app.state.runtime
    try:
        result = await asyncio.to_thread(getattr(runtime.tool_service, method_name), payload)
    except PrivacyPolicyError as error:
        result = ToolRunResponse(
            tool_id=method_name,
            status="blocked",
            title="Blocked by privacy firewall",
            summary=error.response.reason,
            content=error.response.safe_alternative,
            privacy=error.response,
        )
        await publish_tool_event(runtime, "tool.failed", result)
        await publish_privacy_event(runtime, error.response)
        return result
    except ToolExecutionError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    event_type = "tool.permission.required" if result.permission_required else "tool.completed"
    await publish_tool_event(runtime, event_type, result)
    if result.privacy:
        await publish_privacy_event(runtime, result.privacy)
    return result


async def publish_tool_event(runtime, event_type: str, result: ToolRunResponse) -> None:
    await runtime.event_bus.publish(runtime.event(event_type, result.model_dump(mode="json", by_alias=True)))


async def publish_privacy_event(runtime, result) -> None:
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
