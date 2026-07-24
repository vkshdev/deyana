from __future__ import annotations

import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path
from tempfile import gettempdir

from .settings import CoreSettings


def configure_logging(settings: CoreSettings) -> None:
    root = logging.getLogger()
    root.setLevel(settings.log_level.upper())
    root.handlers.clear()

    formatter = logging.Formatter(
        "%(asctime)s %(levelname)s [%(name)s] %(message)s",
        datefmt="%Y-%m-%dT%H:%M:%S%z",
    )

    console_handler = logging.StreamHandler()
    console_handler.setFormatter(formatter)
    root.addHandler(console_handler)

    file_handler = build_file_handler(settings.log_dir) or build_file_handler(
        Path(gettempdir()) / "deyana-core" / "logs"
    )
    if file_handler is not None:
        file_handler.setFormatter(formatter)
        root.addHandler(file_handler)
    else:
        root.warning(
            "file logging is unavailable; continuing with console logging only"
        )


def build_file_handler(log_dir: Path) -> RotatingFileHandler | None:
    try:
        log_dir.mkdir(parents=True, exist_ok=True)
        return RotatingFileHandler(
            log_dir / "core.log",
            maxBytes=1_000_000,
            backupCount=3,
            encoding="utf-8",
        )
    except OSError:
        return None
