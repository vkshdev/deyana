from __future__ import annotations

from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from pydantic import ValidationError

from ..browser import BrowserBridgeAuthenticationError

router = APIRouter(tags=["browser-bridge"])


@router.websocket("/browser/bridge")
async def browser_bridge(websocket: WebSocket) -> None:
    service = websocket.app.state.runtime.browser_service
    try:
        extension_origin = service.authenticate(
            websocket.headers.get("authorization"),
            websocket.headers.get("x-deyana-extension-origin"),
        )
    except BrowserBridgeAuthenticationError:
        await websocket.close(code=4401, reason="Browser bridge authentication failed.")
        return

    await websocket.accept()
    await service.attach(websocket, extension_origin)
    disconnect_reason = "Browser extension disconnected."
    disconnect_is_error = False
    try:
        while True:
            message = await websocket.receive_json()
            await service.handle_bridge_message(message, websocket=websocket)
    except WebSocketDisconnect as error:
        if error.code not in {1000, 1001}:
            disconnect_reason = f"Browser extension connection closed with code {error.code}."
            disconnect_is_error = True
    except ValidationError:
        disconnect_reason = "Browser extension sent an invalid bridge message."
        disconnect_is_error = True
        await websocket.close(code=4400, reason="Invalid browser bridge message.")
    finally:
        await service.detach(
            disconnect_reason,
            websocket=websocket,
            as_error=disconnect_is_error,
        )
