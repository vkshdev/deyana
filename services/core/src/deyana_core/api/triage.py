from fastapi import APIRouter, Request, HTTPException
from typing import Literal
from pydantic import BaseModel

from ..triage import IncomingTriageRequest, TriageMessageResponse

router = APIRouter(prefix="/api/triage", tags=["triage"])

class ResolveRequest(BaseModel):
    status: Literal["approved", "discarded"]

@router.post("/incoming", response_model=dict)
async def incoming_message(request: Request, body: IncomingTriageRequest):
    if request.headers.get("x-deyana-client") != "true" and not request.headers.get("origin"):
        # For strict local API protection against CSRF
        raise HTTPException(status_code=403, detail="Missing required client header or origin")
        
    runtime = request.app.state.runtime
    daemon = runtime.triage_daemon
    if not daemon:
        raise HTTPException(status_code=503, detail="TriageDaemon not available")
    
    daemon.enqueue_message(body)
    return {"status": "enqueued"}

@router.get("/pending", response_model=list[TriageMessageResponse])
async def get_pending_messages(request: Request):
    runtime = request.app.state.runtime
    daemon = runtime.triage_daemon
    if not daemon:
        raise HTTPException(status_code=503, detail="TriageDaemon not available")
    
    return await daemon.get_pending_messages()

@router.post("/{msg_id}/resolve", response_model=dict)
async def resolve_message(request: Request, msg_id: str, body: ResolveRequest):
    runtime = request.app.state.runtime
    daemon = runtime.triage_daemon
    if not daemon:
        raise HTTPException(status_code=503, detail="TriageDaemon not available")
    
    await daemon.resolve_message(msg_id, body.status)
    return {"status": "resolved"}
