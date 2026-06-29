from __future__ import annotations

import asyncio

from fastapi import APIRouter, HTTPException, Request

from ..models import (
    VoiceSettings,
    VoiceSettingsPatch,
    VoiceSpeakRequest,
    VoiceSpeakResponse,
    VoiceStatusResponse,
    VoiceTranscriptRequest,
    VoiceTranscriptResponse,
)
from ..voice import VoiceUnavailableError

router = APIRouter(prefix="/voice", tags=["voice"])


@router.get("/settings", response_model=VoiceSettings)
async def get_voice_settings(request: Request) -> VoiceSettings:
    return request.app.state.runtime.voice_service.read_settings()


@router.patch("/settings", response_model=VoiceSettings)
async def patch_voice_settings(request: Request, payload: VoiceSettingsPatch) -> VoiceSettings:
    runtime = request.app.state.runtime
    try:
        settings = runtime.voice_service.patch_settings(payload)
    except VoiceUnavailableError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error
    await runtime.event_bus.publish(
        runtime.event(
            "voice.settings.updated",
            settings.model_dump(mode="json", by_alias=True),
        )
    )
    return settings


@router.get("/status", response_model=VoiceStatusResponse)
async def get_voice_status(request: Request) -> VoiceStatusResponse:
    return request.app.state.runtime.voice_service.status()


@router.post("/transcribe", response_model=VoiceTranscriptResponse)
async def transcribe_voice(request: Request, payload: VoiceTranscriptRequest) -> VoiceTranscriptResponse:
    runtime = request.app.state.runtime
    await runtime.event_bus.publish(runtime.event("voice.recording.started", {"rawAudioStored": False}))
    await runtime.event_bus.publish(runtime.event("voice.transcription.started", {"engine": "windows_speech"}))

    try:
        result = await asyncio.to_thread(runtime.voice_service.transcribe, payload)
    except (ValueError, VoiceUnavailableError) as error:
        await runtime.event_bus.publish(runtime.event("voice.recording.stopped", {"rawAudioStored": False}))
        await runtime.event_bus.publish(
            runtime.event(
                "voice.transcription.failed",
                {"reason": str(error), "rawAudioStored": False},
            )
        )
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(runtime.event("voice.recording.stopped", {"rawAudioStored": False}))
    await runtime.event_bus.publish(
        runtime.event(
            "voice.transcription.completed",
            result.model_dump(mode="json", by_alias=True),
        )
    )
    return result


@router.post("/speak", response_model=VoiceSpeakResponse)
async def speak_voice(request: Request, payload: VoiceSpeakRequest) -> VoiceSpeakResponse:
    runtime = request.app.state.runtime
    await runtime.event_bus.publish(runtime.event("tts.started", {"engine": "windows_speech"}))

    try:
        result = await asyncio.to_thread(runtime.voice_service.speak, payload)
    except (ValueError, VoiceUnavailableError) as error:
        await runtime.event_bus.publish(
            runtime.event(
                "tts.failed",
                {"reason": str(error), "rawAudioStored": False},
            )
        )
        raise HTTPException(status_code=400, detail=str(error)) from error

    await runtime.event_bus.publish(
        runtime.event(
            "tts.completed",
            result.model_dump(mode="json", by_alias=True),
        )
    )
    return result
