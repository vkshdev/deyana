from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from .models import CoreEvent


class EventBus:
    def __init__(self) -> None:
        self._subscribers: set[asyncio.Queue[CoreEvent]] = set()

    @asynccontextmanager
    async def subscribe(self) -> AsyncIterator[asyncio.Queue[CoreEvent]]:
        queue: asyncio.Queue[CoreEvent] = asyncio.Queue(maxsize=64)
        self._subscribers.add(queue)
        try:
            yield queue
        finally:
            self._subscribers.discard(queue)

    async def publish(self, event: CoreEvent) -> None:
        stale: list[asyncio.Queue[CoreEvent]] = []
        for queue in self._subscribers:
            try:
                queue.put_nowait(event)
            except asyncio.QueueFull:
                stale.append(queue)

        for queue in stale:
            self._subscribers.discard(queue)
