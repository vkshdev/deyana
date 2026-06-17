from __future__ import annotations

from fastapi import APIRouter, HTTPException, Request

from ..models import VaultSelectRequest, VaultSelectResponse

router = APIRouter(tags=["vault"])


@router.post("/vault/select", response_model=VaultSelectResponse)
async def select_vault(request: Request, payload: VaultSelectRequest) -> VaultSelectResponse:
    runtime = request.app.state.runtime
    try:
        state, settings, created_folders = runtime.store.select_vault(payload.path)
    except OSError as error:
        raise HTTPException(status_code=400, detail=f"Unable to create vault: {error}") from error
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "vault.selected",
            {
                "state": state.model_dump(mode="json", by_alias=True),
                "settings": settings.model_dump(mode="json", by_alias=True),
                "vaultPath": state.selected_vault_path,
                "createdFolders": created_folders,
            },
        )
    )
    return VaultSelectResponse(
        state=state,
        settings=settings,
        vault_path=state.selected_vault_path or payload.path,
        created_folders=created_folders,
    )
