from __future__ import annotations

import base64
import ctypes
import hmac
import json
import os
import sqlite3
from ctypes import wintypes
from pathlib import Path
from typing import Any

from .runtime_time import utc_timestamp


class TokenVaultError(RuntimeError):
    pass


class TokenVault:
    def __init__(self, data_dir: Path, database_path: Path | None = None) -> None:
        self.data_dir = data_dir
        self.database_path = database_path or data_dir / "connectors.sqlite3"
        self.key_path = data_dir / "connector-token.key"

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        with self.connect() as connection:
            connection.execute(
                """
                CREATE TABLE IF NOT EXISTS connector_tokens (
                  connector_id TEXT PRIMARY KEY,
                  encrypted_token_json TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                )
                """
            )

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def store(self, connector_id: str, token_payload: dict[str, Any]) -> str:
        self.initialize()
        timestamp = utc_timestamp()
        encrypted = encrypt_token_payload(token_payload, self._local_key)
        with self.connect() as connection:
            with connection:
                existing = connection.execute(
                    "SELECT created_at FROM connector_tokens WHERE connector_id = ?",
                    (connector_id,),
                ).fetchone()
                connection.execute(
                    """
                    INSERT INTO connector_tokens (
                      connector_id, encrypted_token_json, created_at, updated_at
                    )
                    VALUES (?, ?, ?, ?)
                    ON CONFLICT(connector_id) DO UPDATE SET
                      encrypted_token_json = excluded.encrypted_token_json,
                      updated_at = excluded.updated_at
                    """,
                    (
                        connector_id,
                        encrypted,
                        existing["created_at"] if existing else timestamp,
                        timestamp,
                    ),
                )
        return timestamp

    def read(self, connector_id: str) -> dict[str, Any] | None:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT encrypted_token_json FROM connector_tokens WHERE connector_id = ?",
                (connector_id,),
            ).fetchone()
        if not row:
            return None
        return decrypt_token_payload(row["encrypted_token_json"], self._local_key)

    def delete(self, connector_id: str) -> bool:
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute(
                    "DELETE FROM connector_tokens WHERE connector_id = ?",
                    (connector_id,),
                )
        return cursor.rowcount > 0

    def has_token(self, connector_id: str) -> bool:
        self.initialize()
        with self.connect() as connection:
            row = connection.execute(
                "SELECT 1 FROM connector_tokens WHERE connector_id = ?",
                (connector_id,),
            ).fetchone()
        return row is not None

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


def encrypt_token_payload(payload: dict[str, Any], local_key_factory: callable[[], bytes]) -> str:
    plaintext = json.dumps(payload, separators=(",", ":"), sort_keys=True).encode("utf-8")
    if os.name == "nt":
        encrypted = _dpapi_protect(plaintext)
        return json.dumps(
            {
                "version": 1,
                "provider": "windows-dpapi",
                "payload": base64.b64encode(encrypted).decode("ascii"),
            },
            separators=(",", ":"),
        )

    encrypted = _hmac_stream_encrypt(plaintext, local_key_factory())
    return json.dumps({"version": 1, "provider": "local-hmac-stream", **encrypted}, separators=(",", ":"))


def decrypt_token_payload(encrypted_token_json: str, local_key_factory: callable[[], bytes]) -> dict[str, Any]:
    envelope = json.loads(encrypted_token_json)
    provider = envelope.get("provider")
    if provider == "windows-dpapi":
        plaintext = _dpapi_unprotect(base64.b64decode(envelope["payload"]))
    elif provider == "local-hmac-stream":
        plaintext = _hmac_stream_decrypt(envelope, local_key_factory())
    else:
        raise TokenVaultError("Unsupported token encryption provider.")
    return json.loads(plaintext.decode("utf-8"))


class _DataBlob(ctypes.Structure):
    _fields_ = [
        ("cbData", wintypes.DWORD),
        ("pbData", ctypes.POINTER(ctypes.c_char)),
    ]


def _dpapi_protect(plaintext: bytes) -> bytes:
    return _dpapi_call(plaintext, protect=True)


def _dpapi_unprotect(ciphertext: bytes) -> bytes:
    return _dpapi_call(ciphertext, protect=False)


def _dpapi_call(data: bytes, *, protect: bool) -> bytes:
    crypt32 = ctypes.windll.crypt32
    kernel32 = ctypes.windll.kernel32
    input_buffer = ctypes.create_string_buffer(data)
    input_blob = _DataBlob(len(data), ctypes.cast(input_buffer, ctypes.POINTER(ctypes.c_char)))
    output_blob = _DataBlob()

    if protect:
        ok = crypt32.CryptProtectData(
            ctypes.byref(input_blob),
            "DEYANA connector token",
            None,
            None,
            None,
            0,
            ctypes.byref(output_blob),
        )
    else:
        ok = crypt32.CryptUnprotectData(
            ctypes.byref(input_blob),
            None,
            None,
            None,
            None,
            0,
            ctypes.byref(output_blob),
        )

    if not ok:
        raise TokenVaultError("Windows DPAPI token encryption failed.")

    try:
        return ctypes.string_at(output_blob.pbData, output_blob.cbData)
    finally:
        kernel32.LocalFree(output_blob.pbData)


def _hmac_stream_encrypt(plaintext: bytes, key: bytes) -> dict[str, str]:
    nonce = os.urandom(16)
    ciphertext = xor_bytes(plaintext, _keystream(key, nonce, len(plaintext)))
    tag = hmac.digest(key, b"deyana-token-v1" + nonce + ciphertext, "sha256")
    return {
        "nonce": base64.b64encode(nonce).decode("ascii"),
        "ciphertext": base64.b64encode(ciphertext).decode("ascii"),
        "tag": base64.b64encode(tag).decode("ascii"),
    }


def _hmac_stream_decrypt(envelope: dict[str, Any], key: bytes) -> bytes:
    nonce = base64.b64decode(envelope["nonce"])
    ciphertext = base64.b64decode(envelope["ciphertext"])
    tag = base64.b64decode(envelope["tag"])
    expected = hmac.digest(key, b"deyana-token-v1" + nonce + ciphertext, "sha256")
    if not hmac.compare_digest(tag, expected):
        raise TokenVaultError("Connector token integrity check failed.")
    return xor_bytes(ciphertext, _keystream(key, nonce, len(ciphertext)))


def _keystream(key: bytes, nonce: bytes, length: int) -> bytes:
    output = bytearray()
    counter = 0
    while len(output) < length:
        output.extend(hmac.digest(key, nonce + counter.to_bytes(8, "big"), "sha256"))
        counter += 1
    return bytes(output[:length])


def xor_bytes(left: bytes, right: bytes) -> bytes:
    return bytes(left_byte ^ right_byte for left_byte, right_byte in zip(left, right, strict=True))
