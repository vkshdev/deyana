import asyncio
import logging
import uuid
import sqlite3
from typing import Optional, Literal
from pydantic import BaseModel
from datetime import timezone, datetime

from .local_models import ModelRouter
from .runtime_time import utc_timestamp

logger = logging.getLogger(__name__)

class IncomingTriageRequest(BaseModel):
    platform: str
    sender: str
    content: str

class TriageMessageResponse(BaseModel):
    id: str
    platform: str
    sender: str
    content: str
    urgency_score: str
    auto_draft: str
    status: str
    created_at: str

class TriageDaemon:
    def __init__(self, model_router: ModelRouter, db_path: str):
        self.model_router = model_router
        self.db_path = db_path
        self._queue: asyncio.Queue[IncomingTriageRequest] = asyncio.Queue()
        self._task: Optional[asyncio.Task] = None

    def start(self):
        if self._task is None:
            self._task = asyncio.create_task(self._process_queue())
            logger.info("TriageDaemon started")

    def stop(self):
        if self._task:
            self._task.cancel()
            self._task = None
            logger.info("TriageDaemon stopped")

    def enqueue_message(self, message: IncomingTriageRequest):
        self._queue.put_nowait(message)

    async def _process_queue(self):
        try:
            while True:
                request = await self._queue.get()
                try:
                    await self._process_single_message(request)
                except Exception as e:
                    logger.error(f"Error processing triage message: {e}")
                finally:
                    self._queue.task_done()
        except asyncio.CancelledError:
            pass

    async def _process_single_message(self, request: IncomingTriageRequest):
        logger.info(f"TriageDaemon processing message from {request.sender} on {request.platform}")
        
        prompt = (
            f"You are an AI inbox assistant analyzing an incoming message.\n"
            f"Platform: {request.platform}\n"
            f"Sender: {request.sender}\n"
            f"Message: {request.content}\n\n"
            f"Please output a strict JSON with exactly two fields:\n"
            f"1. 'urgency_score': MUST be exactly one of 'URGENT', 'NORMAL', or 'SPAM'\n"
            f"2. 'auto_draft': A draft reply to the user, or empty if it's SPAM.\n\n"
            f"Respond ONLY with valid JSON."
        )

        try:
            # We must use asyncio.to_thread because generation might block
            generation = await asyncio.to_thread(
                self.model_router.generate_prompt,
                prompt,
                temperature=0.1,
                num_predict=300
            )
            response_text = generation.response.strip()
            
            # Simple JSON extraction
            import json
            import re
            
            try:
                data = json.loads(response_text)
            except json.JSONDecodeError:
                json_match = re.search(r'\{.*\}', response_text, re.DOTALL)
                if json_match:
                    data = json.loads(json_match.group(0))
                else:
                    data = {}
                
            urgency = data.get("urgency_score", "NORMAL").upper()
            if urgency not in ["URGENT", "NORMAL", "SPAM"]:
                urgency = "NORMAL"
            draft = data.get("auto_draft", "")
            
        except Exception as e:
            logger.error(f"Failed to score message: {e}")
            urgency = "NORMAL"
            draft = "Error generating draft."

        self._save_triage_result(request, urgency, draft)

    def _save_triage_result(self, request: IncomingTriageRequest, urgency: str, draft: str):
        msg_id = f"triage_{uuid.uuid4().hex}"
        timestamp = utc_timestamp()
        try:
            with sqlite3.connect(self.db_path) as conn:
                conn.execute(
                    """
                    INSERT INTO triage_inbox 
                    (id, platform, sender, content, urgency_score, auto_draft, status, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (msg_id, request.platform, request.sender, request.content, urgency, draft, 'pending', timestamp)
                )
        except Exception as e:
            logger.error(f"Failed to save triage result to DB: {e}")

    def get_pending_messages(self) -> list[TriageMessageResponse]:
        messages = []
        try:
            with sqlite3.connect(self.db_path) as conn:
                conn.row_factory = sqlite3.Row
                cursor = conn.execute("SELECT * FROM triage_inbox WHERE status = 'pending' ORDER BY created_at DESC")
                for row in cursor:
                    messages.append(TriageMessageResponse(**dict(row)))
        except Exception as e:
            logger.error(f"Failed to get pending messages: {e}")
        return messages

    def resolve_message(self, msg_id: str, status: Literal["approved", "discarded"]):
        try:
            with sqlite3.connect(self.db_path) as conn:
                conn.execute("UPDATE triage_inbox SET status = ? WHERE id = ?", (status, msg_id))
        except Exception as e:
            logger.error(f"Failed to update triage status: {e}")
