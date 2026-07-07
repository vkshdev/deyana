from __future__ import annotations

import json
import os
import secrets
from pathlib import Path

from ..runtime_time import utc_timestamp
from ..token_vault import TokenVaultError, decrypt_token_payload, encrypt_token_payload


class BrowserCredentialError(RuntimeError):
    pass


class BrowserBridgeCredentialStore:
    def __init__(self, data_dir: Path, endpoint: str) -> None:
        self.data_dir = data_dir
        self.endpoint = endpoint
        self.path = data_dir / "browser-bridge-credential.json"
        self.key_path = data_dir / "browser-bridge.key"

    def initialize(self) -> str:
        existing = self.read()
        if existing:
            return existing

        token = secrets.token_urlsafe(48)
        encrypted = encrypt_token_payload({"token": token}, self._local_key)
        self._write(
            {
                "schemaVersion": 1,
                "endpoint": self.endpoint,
                "encryptedCredential": encrypted,
                "createdAt": utc_timestamp(),
            }
        )
        return token

    def read(self) -> str | None:
        try:
            envelope = json.loads(self.path.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            return None

        try:
            payload = decrypt_token_payload(envelope["encryptedCredential"], self._local_key)
        except (KeyError, TypeError, ValueError, TokenVaultError) as error:
            raise BrowserCredentialError("Browser bridge credential is invalid.") from error
        token = payload.get("token")
        if not isinstance(token, str) or len(token) < 32:
            raise BrowserCredentialError("Browser bridge credential token is invalid.")
        return token

    def rotate(self) -> str:
        try:
            self.path.unlink()
        except FileNotFoundError:
            pass
        return self.initialize()

    def delete(self) -> bool:
        deleted = False
        for path in (self.path, self.key_path):
            try:
                path.unlink()
                deleted = True
            except FileNotFoundError:
                pass
        return deleted

    def _local_key(self) -> bytes:
        if self.key_path.exists():
            return self.key_path.read_bytes()
        key = os.urandom(32)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.key_path.write_bytes(key)
        try:
            os.chmod(self.key_path, 0o600)
        except OSError:
            pass
        return key

    def _write(self, payload: dict[str, object]) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        temp_path = self.path.with_suffix(".json.tmp")
        temp_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        temp_path.replace(self.path)
        try:
            os.chmod(self.path, 0o600)
        except OSError:
            pass
