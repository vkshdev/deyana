from __future__ import annotations

import json
import os
import sqlite3
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen

from .memory import MemoryStore
from .models import (
    ConnectorItem,
    ConnectorListResponse,
    ConnectorOAuthStartResponse,
    ConnectorSyncResponse,
    ConnectorSyncRun,
    ConnectorSyncRunsResponse,
    ConnectorSyncRunStatus,
    ConnectorStatus,
    MemoryCreateRequest,
    PrivacyCheckRequest,
    PrivacyCheckResponse,
)
from .privacy import PrivacyFirewall, PrivacyPolicyError
from .runtime_time import utc_timestamp
from .token_vault import TokenVault


DEFAULT_SYNC_INTERVAL_MINUTES = 360


class ConnectorError(RuntimeError):
    pass


class ConnectorNotFoundError(ConnectorError):
    pass


class ConnectorStateError(ConnectorError):
    pass


class ConnectorHttpError(ConnectorError):
    pass


@dataclass(frozen=True)
class ConnectorRecord:
    external_id: str
    title: str
    summary: str
    content_markdown: str
    source_uri: str | None
    item_timestamp: str | None
    tags: tuple[str, ...]
    normalized: dict[str, object]


@dataclass(frozen=True)
class ConnectorSyncResult:
    items_seen: int
    records: tuple[ConnectorRecord, ...]
    detail: str


@dataclass(frozen=True)
class ConnectorDefinition:
    id: str
    name: str
    scopes: tuple[str, ...]
    authorization_url: str
    token_url: str
    api_base_url: str
    api_probe_path: str
    client_id_env: str | None
    client_secret_env: str | None
    token_url_env: str | None
    api_base_url_env: str | None
    default_sync_interval_minutes: int = DEFAULT_SYNC_INTERVAL_MINUTES


@dataclass(frozen=True)
class ConnectorSyncContext:
    token: dict[str, object]
    http_client: "ConnectorHttpClient"


class BaseConnector:
    definition: ConnectorDefinition

    @property
    def id(self) -> str:
        return self.definition.id

    @property
    def name(self) -> str:
        return self.definition.name

    @property
    def scopes(self) -> list[str]:
        return list(self.definition.scopes)

    def oauth_configured(self) -> bool:
        return bool(self.oauth_client_id() and self.oauth_client_secret())

    def oauth_client_id(self) -> str | None:
        if not self.definition.client_id_env:
            return None
        return os.getenv(self.definition.client_id_env)

    def oauth_client_secret(self) -> str | None:
        if not self.definition.client_secret_env:
            return None
        return os.getenv(self.definition.client_secret_env)

    def token_url(self) -> str:
        if self.definition.token_url_env:
            return os.getenv(self.definition.token_url_env, self.definition.token_url)
        return self.definition.token_url

    def api_base_url(self) -> str:
        if self.definition.api_base_url_env:
            return os.getenv(self.definition.api_base_url_env, self.definition.api_base_url).rstrip("/")
        return self.definition.api_base_url.rstrip("/")

    def api_probe_url(self) -> str:
        return f"{self.api_base_url()}{self.definition.api_probe_path}"

    def build_authorization_url(self, *, state: str, redirect_uri: str) -> str:
        client_id = self.oauth_client_id() or "deyana-local-mock"
        query = urlencode(
            {
                "client_id": client_id,
                "redirect_uri": redirect_uri,
                "response_type": "code",
                "scope": " ".join(self.definition.scopes),
                "state": state,
                "access_type": "offline",
                "prompt": "consent",
            }
        )
        return f"{self.definition.authorization_url}?{query}"

    def exchange_oauth_code(
        self,
        *,
        code: str,
        redirect_uri: str,
        issued_at: str,
        http_client: "ConnectorHttpClient",
        user_approved: bool,
    ) -> dict[str, object]:
        client_id = self.oauth_client_id()
        client_secret = self.oauth_client_secret()
        if not client_id or not client_secret:
            return self.create_mock_token(code=code, issued_at=issued_at)

        response = http_client.post_form(
            self.token_url(),
            {
                "client_id": client_id,
                "client_secret": client_secret,
                "code": code,
                "redirect_uri": redirect_uri,
                "grant_type": "authorization_code",
            },
            purpose="oauth_api_fetch",
            data_category="oauth_token",
            payload_preview=f"{self.name} OAuth code exchange",
            user_approved=user_approved,
            headers={"accept": "application/json"},
        )
        return self.token_payload_from_response(response, issued_at=issued_at)

    def create_mock_token(self, *, code: str, issued_at: str) -> dict[str, object]:
        _ = code
        return {
            "schemaVersion": 1,
            "connectorId": self.id,
            "accessToken": f"mock_access_{uuid.uuid4().hex}",
            "refreshToken": f"mock_refresh_{uuid.uuid4().hex}",
            "tokenType": "Bearer",
            "scopes": self.scopes,
            "issuedAt": issued_at,
            "expiresAt": (datetime.now(UTC) + timedelta(hours=1)).isoformat().replace("+00:00", "Z"),
            "mock": True,
        }

    def token_payload_from_response(self, response: dict[str, object], *, issued_at: str) -> dict[str, object]:
        access_token = response.get("access_token")
        if not isinstance(access_token, str) or not access_token:
            raise ConnectorStateError("OAuth provider did not return an access token.")

        expires_in = response.get("expires_in")
        expires_at = None
        if isinstance(expires_in, int):
            expires_at = (datetime.now(UTC) + timedelta(seconds=expires_in)).isoformat().replace("+00:00", "Z")

        return {
            "schemaVersion": 1,
            "connectorId": self.id,
            "accessToken": access_token,
            "refreshToken": response.get("refresh_token"),
            "tokenType": response.get("token_type", "Bearer"),
            "scopes": response.get("scope", " ".join(self.scopes)),
            "issuedAt": issued_at,
            "expiresAt": expires_at,
            "mock": False,
        }

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        self.validate_token(context.token)
        if context.token.get("mock"):
            return ConnectorSyncResult(
                items_seen=0,
                records=(),
                detail="Mock connector token is connected; configure real OAuth credentials to fetch live data.",
            )
        raise ConnectorStateError(f"{self.name} real sync is not implemented.")

    def validate_token(self, token: dict[str, object]) -> None:
        if token.get("connectorId") != self.id:
            raise ConnectorStateError("Stored token belongs to a different connector.")


class GmailConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        base_url = self.api_base_url()
        list_url = f"{base_url}/users/me/messages?{urlencode({'maxResults': 10})}"
        listing = context.http_client.get_json(
            list_url,
            token=context.token,
            payload_preview="Gmail message metadata list",
        )
        messages = listing.get("messages") if isinstance(listing, dict) else []
        records: list[ConnectorRecord] = []
        for message in messages if isinstance(messages, list) else []:
            message_id = message.get("id") if isinstance(message, dict) else None
            if not isinstance(message_id, str) or not message_id:
                continue
            query = urlencode(
                {
                    "format": "metadata",
                    "metadataHeaders": ["From", "Subject", "Date"],
                },
                doseq=True,
            )
            detail = context.http_client.get_json(
                f"{base_url}/users/me/messages/{quote(message_id, safe='')}?{query}",
                token=context.token,
                payload_preview="Gmail message metadata detail",
            )
            records.append(gmail_record(detail, message_id))
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Gmail metadata synced.")


class CalendarConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        time_min = (datetime.now(UTC) - timedelta(days=1)).isoformat().replace("+00:00", "Z")
        query = urlencode(
            {
                "maxResults": 20,
                "singleEvents": "true",
                "orderBy": "startTime",
                "timeMin": time_min,
            }
        )
        response = context.http_client.get_json(
            f"{self.api_base_url()}/calendars/primary/events?{query}",
            token=context.token,
            payload_preview="Calendar event metadata list",
        )
        items = response.get("items") if isinstance(response, dict) else []
        records = [
            calendar_record(item)
            for item in items
            if isinstance(item, dict) and isinstance(item.get("id"), str)
        ]
        return ConnectorSyncResult(
            items_seen=len(records),
            records=tuple(records),
            detail="Google Calendar events synced.",
        )


class GitHubConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def exchange_oauth_code(
        self,
        *,
        code: str,
        redirect_uri: str,
        issued_at: str,
        http_client: "ConnectorHttpClient",
        user_approved: bool,
    ) -> dict[str, object]:
        client_id = self.oauth_client_id()
        client_secret = self.oauth_client_secret()
        if not client_id or not client_secret:
            return self.create_mock_token(code=code, issued_at=issued_at)

        response = http_client.post_form(
            self.token_url(),
            {
                "client_id": client_id,
                "client_secret": client_secret,
                "code": code,
                "redirect_uri": redirect_uri,
            },
            purpose="oauth_api_fetch",
            data_category="oauth_token",
            payload_preview="GitHub OAuth code exchange",
            user_approved=user_approved,
            headers={"accept": "application/json"},
        )
        return self.token_payload_from_response(response, issued_at=issued_at)

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        query = urlencode({"per_page": 20, "sort": "updated", "direction": "desc"})
        response = context.http_client.get_json(
            f"{self.api_base_url()}/user/repos?{query}",
            token=context.token,
            payload_preview="GitHub repository metadata list",
            headers={"accept": "application/vnd.github+json"},
        )
        repos = response if isinstance(response, list) else response.get("items", [])
        records = [
            github_record(repo)
            for repo in repos
            if isinstance(repo, dict) and (repo.get("id") or repo.get("full_name"))
        ]
        return ConnectorSyncResult(
            items_seen=len(records),
            records=tuple(records),
            detail="GitHub repositories synced.",
        )


class DriveConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        query = urlencode({"pageSize": 20, "fields": "files(id,name,mimeType,modifiedTime,webViewLink)"})
        response = context.http_client.get_json(
            f"{self.api_base_url()}/files?{query}",
            token=context.token,
            payload_preview="Google Drive file metadata list",
        )
        files = response.get("files") if isinstance(response, dict) else []
        records = [drive_record(item) for item in files if isinstance(item, dict) and isinstance(item.get("id"), str)]
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Google Drive files synced.")


class SlackConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        channels_response = context.http_client.get_json(
            f"{self.api_base_url()}/conversations.list?{urlencode({'limit': 10, 'types': 'public_channel,private_channel'})}",
            token=context.token,
            payload_preview="Slack channel metadata list",
        )
        channels = channels_response.get("channels") if isinstance(channels_response, dict) else []
        records: list[ConnectorRecord] = []
        for channel in channels if isinstance(channels, list) else []:
            channel_id = channel.get("id") if isinstance(channel, dict) else None
            channel_name = channel.get("name") if isinstance(channel, dict) else None
            if not isinstance(channel_id, str):
                continue
            response = context.http_client.get_json(
                f"{self.api_base_url()}/conversations.history?{urlencode({'channel': channel_id, 'limit': 5})}",
                token=context.token,
                payload_preview="Slack message metadata list",
            )
            messages = response.get("messages") if isinstance(response, dict) else []
            records.extend(
                slack_record(item, channel_name if isinstance(channel_name, str) else channel_id)
                for item in messages
                if isinstance(item, dict) and item.get("ts")
            )
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Slack messages synced.")


class NotionConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        response = context.http_client.post_json(
            f"{self.api_base_url()}/search",
            {"page_size": 20},
            token=context.token,
            payload_preview="Notion page metadata search",
            headers={"notion-version": "2022-06-28"},
        )
        results = response.get("results") if isinstance(response, dict) else []
        records = [notion_record(item) for item in results if isinstance(item, dict) and isinstance(item.get("id"), str)]
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Notion pages synced.")


class JiraConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        query = urlencode({"jql": "updated >= -14d ORDER BY updated DESC", "maxResults": 20})
        response = context.http_client.get_json(
            f"{self.api_base_url()}/search?{query}",
            token=context.token,
            payload_preview="Jira issue metadata search",
        )
        issues = response.get("issues") if isinstance(response, dict) else []
        records = [jira_record(item, self.api_base_url()) for item in issues if isinstance(item, dict) and item.get("id")]
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Jira issues synced.")


class LinearConnector(BaseConnector):
    def __init__(self, definition: ConnectorDefinition) -> None:
        self.definition = definition

    def sync(self, context: ConnectorSyncContext) -> ConnectorSyncResult:
        if context.token.get("mock"):
            return super().sync(context)
        self.validate_token(context.token)
        response = context.http_client.post_json(
            self.api_base_url(),
            {
                "query": (
                    "query DeyanaIssues { issues(first: 20, orderBy: updatedAt) "
                    "{ nodes { id identifier title url updatedAt state { name } assignee { name } } } }"
                )
            },
            token=context.token,
            payload_preview="Linear issue metadata query",
        )
        nodes = response.get("data", {}).get("issues", {}).get("nodes", []) if isinstance(response, dict) else []
        records = [linear_record(item) for item in nodes if isinstance(item, dict) and isinstance(item.get("id"), str)]
        return ConnectorSyncResult(items_seen=len(records), records=tuple(records), detail="Linear issues synced.")


CONNECTOR_DEFINITIONS = [
    ConnectorDefinition(
        id="gmail",
        name="Gmail",
        scopes=("https://www.googleapis.com/auth/gmail.readonly",),
        authorization_url="https://accounts.google.com/o/oauth2/v2/auth",
        token_url="https://oauth2.googleapis.com/token",
        api_base_url="https://gmail.googleapis.com/gmail/v1",
        api_probe_path="/users/me/messages",
        client_id_env="DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_GOOGLE_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_GMAIL_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="calendar",
        name="Calendar",
        scopes=("https://www.googleapis.com/auth/calendar.readonly",),
        authorization_url="https://accounts.google.com/o/oauth2/v2/auth",
        token_url="https://oauth2.googleapis.com/token",
        api_base_url="https://www.googleapis.com/calendar/v3",
        api_probe_path="/calendars/primary/events",
        client_id_env="DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_GOOGLE_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_CALENDAR_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="github",
        name="GitHub",
        scopes=("read:user", "repo"),
        authorization_url="https://github.com/login/oauth/authorize",
        token_url="https://github.com/login/oauth/access_token",
        api_base_url="https://api.github.com",
        api_probe_path="/user/repos",
        client_id_env="DEYANA_GITHUB_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_GITHUB_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_GITHUB_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_GITHUB_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="drive",
        name="Google Drive",
        scopes=("https://www.googleapis.com/auth/drive.metadata.readonly",),
        authorization_url="https://accounts.google.com/o/oauth2/v2/auth",
        token_url="https://oauth2.googleapis.com/token",
        api_base_url="https://www.googleapis.com/drive/v3",
        api_probe_path="/files",
        client_id_env="DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_GOOGLE_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_DRIVE_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="slack",
        name="Slack",
        scopes=("channels:read", "channels:history"),
        authorization_url="https://slack.com/oauth/v2/authorize",
        token_url="https://slack.com/api/oauth.v2.access",
        api_base_url="https://slack.com/api",
        api_probe_path="/auth.test",
        client_id_env="DEYANA_SLACK_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_SLACK_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_SLACK_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_SLACK_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="notion",
        name="Notion",
        scopes=("read_content",),
        authorization_url="https://api.notion.com/v1/oauth/authorize",
        token_url="https://api.notion.com/v1/oauth/token",
        api_base_url="https://api.notion.com/v1",
        api_probe_path="/search",
        client_id_env="DEYANA_NOTION_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_NOTION_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_NOTION_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_NOTION_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="jira",
        name="Jira",
        scopes=("read:jira-work", "offline_access"),
        authorization_url="https://auth.atlassian.com/authorize",
        token_url="https://auth.atlassian.com/oauth/token",
        api_base_url="https://api.atlassian.com/ex/jira",
        api_probe_path="/rest/api/3/search",
        client_id_env="DEYANA_JIRA_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_JIRA_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_JIRA_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_JIRA_API_BASE_URL",
    ),
    ConnectorDefinition(
        id="linear",
        name="Linear",
        scopes=("read",),
        authorization_url="https://linear.app/oauth/authorize",
        token_url="https://api.linear.app/oauth/token",
        api_base_url="https://api.linear.app/graphql",
        api_probe_path="",
        client_id_env="DEYANA_LINEAR_OAUTH_CLIENT_ID",
        client_secret_env="DEYANA_LINEAR_OAUTH_CLIENT_SECRET",
        token_url_env="DEYANA_LINEAR_OAUTH_TOKEN_URL",
        api_base_url_env="DEYANA_LINEAR_API_BASE_URL",
    ),
]


CONNECTOR_CLASSES = {
    "gmail": GmailConnector,
    "calendar": CalendarConnector,
    "github": GitHubConnector,
    "drive": DriveConnector,
    "slack": SlackConnector,
    "notion": NotionConnector,
    "jira": JiraConnector,
    "linear": LinearConnector,
}


class ConnectorHttpClient:
    def __init__(self, privacy_firewall: PrivacyFirewall, connector_id: str) -> None:
        self.privacy_firewall = privacy_firewall
        self.connector_id = connector_id
        self.privacy_results: list[PrivacyCheckResponse] = []

    def get_json(
        self,
        url: str,
        *,
        token: dict[str, object],
        payload_preview: str,
        headers: dict[str, str] | None = None,
    ) -> dict[str, Any] | list[Any]:
        request_headers = {
            "accept": "application/json",
            "authorization": f"Bearer {token.get('accessToken', '')}",
            **(headers or {}),
        }
        return self._request_json(
            "GET",
            url,
            purpose="connector_api_fetch",
            data_category="connector_metadata",
            payload_preview=payload_preview,
            user_approved=True,
            headers=request_headers,
        )

    def post_form(
        self,
        url: str,
        data: dict[str, object],
        *,
        purpose: str,
        data_category: str,
        payload_preview: str,
        user_approved: bool,
        headers: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        body = urlencode(data).encode("utf-8")
        response = self._request_json(
            "POST",
            url,
            body=body,
            purpose=purpose,
            data_category=data_category,
            payload_preview=payload_preview,
            user_approved=user_approved,
            headers={
                "content-type": "application/x-www-form-urlencoded",
                "accept": "application/json",
                **(headers or {}),
            },
        )
        if not isinstance(response, dict):
            raise ConnectorHttpError("OAuth provider returned an invalid response.")
        return response

    def post_json(
        self,
        url: str,
        data: dict[str, object],
        *,
        token: dict[str, object],
        payload_preview: str,
        headers: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        response = self._request_json(
            "POST",
            url,
            body=json.dumps(data).encode("utf-8"),
            purpose="connector_api_fetch",
            data_category="connector_metadata",
            payload_preview=payload_preview,
            user_approved=True,
            headers={
                "content-type": "application/json",
                "accept": "application/json",
                "authorization": f"Bearer {token.get('accessToken', '')}",
                **(headers or {}),
            },
        )
        if not isinstance(response, dict):
            raise ConnectorHttpError("Connector returned an invalid JSON response.")
        return response

    def _request_json(
        self,
        method: str,
        url: str,
        *,
        purpose: str,
        data_category: str,
        payload_preview: str,
        user_approved: bool,
        headers: dict[str, str],
        body: bytes | None = None,
    ) -> dict[str, Any] | list[Any]:
        privacy_result = self.privacy_firewall.check(
            PrivacyCheckRequest(
                url=url,
                method=method,
                purpose=purpose,
                data_category=data_category,
                payload_preview=payload_preview,
                user_approved=user_approved,
                connector_id=self.connector_id,
            )
        )
        self.privacy_results.append(privacy_result)
        if not privacy_result.allowed:
            raise PrivacyPolicyError(privacy_result)

        request = Request(url, data=body, headers=headers, method=method)
        try:
            with urlopen(request, timeout=20) as response:
                raw_body = response.read().decode("utf-8")
        except HTTPError as error:
            raise ConnectorHttpError(f"Connector request failed with HTTP {error.code}.") from error
        except URLError as error:
            raise ConnectorHttpError(f"Connector request failed: {error.reason}") from error

        if not raw_body:
            return {}
        try:
            parsed = json.loads(raw_body)
        except json.JSONDecodeError as error:
            raise ConnectorHttpError("Connector returned invalid JSON.") from error
        if not isinstance(parsed, (dict, list)):
            raise ConnectorHttpError("Connector returned an unsupported JSON payload.")
        return parsed


def gmail_record(payload: dict[str, Any], fallback_id: str) -> ConnectorRecord:
    message_id = str(payload.get("id") or fallback_id)
    headers = headers_by_name(payload)
    subject = headers.get("subject") or f"Gmail message {message_id}"
    sender = headers.get("from") or "Unknown sender"
    sent_at = headers.get("date")
    snippet = str(payload.get("snippet") or "").strip()
    summary = compact_sentence(f"Email from {sender}: {subject}. {snippet}")
    content = "\n".join(
        [
            "## Gmail summary",
            "",
            f"- From: {sender}",
            f"- Subject: {subject}",
            f"- Date: {sent_at or 'Unknown'}",
            f"- Message ID: {message_id}",
            "",
            snippet or "No snippet was returned by Gmail metadata sync.",
        ]
    )
    return ConnectorRecord(
        external_id=message_id,
        title=f"Gmail: {subject}",
        summary=summary,
        content_markdown=content,
        source_uri=f"https://mail.google.com/mail/u/0/#all/{message_id}",
        item_timestamp=sent_at,
        tags=("connector", "gmail", "email"),
        normalized={
            "messageId": message_id,
            "from": sender,
            "subject": subject,
            "date": sent_at,
            "snippet": snippet,
        },
    )


def calendar_record(payload: dict[str, Any]) -> ConnectorRecord:
    event_id = str(payload["id"])
    title = str(payload.get("summary") or f"Calendar event {event_id}")
    start = event_time(payload.get("start"))
    end = event_time(payload.get("end"))
    location = str(payload.get("location") or "").strip()
    description = str(payload.get("description") or "").strip()
    summary = compact_sentence(
        f"Calendar event {title} starts {start or 'at an unknown time'}"
        + (f" at {location}." if location else ".")
    )
    content_lines = [
        "## Calendar summary",
        "",
        f"- Event: {title}",
        f"- Start: {start or 'Unknown'}",
        f"- End: {end or 'Unknown'}",
        f"- Location: {location or 'Not provided'}",
    ]
    if description:
        content_lines.extend(["", description])
    return ConnectorRecord(
        external_id=event_id,
        title=f"Calendar: {title}",
        summary=summary,
        content_markdown="\n".join(content_lines),
        source_uri=payload.get("htmlLink") if isinstance(payload.get("htmlLink"), str) else None,
        item_timestamp=start,
        tags=("connector", "calendar", "event"),
        normalized={
            "eventId": event_id,
            "summary": title,
            "start": start,
            "end": end,
            "location": location,
        },
    )


def github_record(payload: dict[str, Any]) -> ConnectorRecord:
    external_id = str(payload.get("id") or payload.get("full_name"))
    full_name = str(payload.get("full_name") or payload.get("name") or external_id)
    description = str(payload.get("description") or "").strip()
    language = str(payload.get("language") or "").strip()
    updated_at = payload.get("updated_at") if isinstance(payload.get("updated_at"), str) else None
    visibility = "private" if payload.get("private") else "public"
    summary = compact_sentence(
        f"GitHub repository {full_name} is {visibility}"
        + (f", uses {language}" if language else "")
        + (f", and was updated {updated_at}" if updated_at else "")
        + "."
    )
    content = "\n".join(
        [
            "## GitHub repository summary",
            "",
            f"- Repository: {full_name}",
            f"- Visibility: {visibility}",
            f"- Language: {language or 'Not detected'}",
            f"- Updated: {updated_at or 'Unknown'}",
            "",
            description or "No repository description was provided.",
        ]
    )
    return ConnectorRecord(
        external_id=external_id,
        title=f"GitHub: {full_name}",
        summary=summary,
        content_markdown=content,
        source_uri=payload.get("html_url") if isinstance(payload.get("html_url"), str) else None,
        item_timestamp=updated_at,
        tags=("connector", "github", "repository"),
        normalized={
            "repositoryId": external_id,
            "fullName": full_name,
            "private": bool(payload.get("private")),
            "language": language,
            "updatedAt": updated_at,
        },
    )


def drive_record(payload: dict[str, Any]) -> ConnectorRecord:
    file_id = str(payload["id"])
    name = str(payload.get("name") or f"Drive file {file_id}")
    mime_type = str(payload.get("mimeType") or "unknown")
    modified = payload.get("modifiedTime") if isinstance(payload.get("modifiedTime"), str) else None
    summary = compact_sentence(f"Google Drive file {name} has type {mime_type} and was modified {modified or 'at an unknown time'}.")
    return ConnectorRecord(
        external_id=file_id,
        title=f"Drive: {name}",
        summary=summary,
        content_markdown="\n".join(
            [
                "## Google Drive file summary",
                "",
                f"- File: {name}",
                f"- MIME type: {mime_type}",
                f"- Modified: {modified or 'Unknown'}",
            ]
        ),
        source_uri=payload.get("webViewLink") if isinstance(payload.get("webViewLink"), str) else None,
        item_timestamp=modified,
        tags=("connector", "drive", "file"),
        normalized={"fileId": file_id, "name": name, "mimeType": mime_type, "modifiedTime": modified},
    )


def slack_record(payload: dict[str, Any], channel: str) -> ConnectorRecord:
    timestamp = str(payload.get("ts"))
    text = str(payload.get("text") or "").strip()
    user = str(payload.get("user") or payload.get("username") or "unknown")
    summary = compact_sentence(f"Slack message in {channel} from {user}: {text or 'No message text returned.'}")
    return ConnectorRecord(
        external_id=f"{channel}:{timestamp}",
        title=f"Slack: {channel} - {compact_sentence(text or timestamp, 80)}",
        summary=summary,
        content_markdown="\n".join(
            ["## Slack message summary", "", f"- Channel: {channel}", f"- User: {user}", f"- Timestamp: {timestamp}", "", text]
        ),
        source_uri=None,
        item_timestamp=timestamp,
        tags=("connector", "slack", "message"),
        normalized={"timestamp": timestamp, "channel": channel, "user": user, "text": text},
    )


def notion_record(payload: dict[str, Any]) -> ConnectorRecord:
    page_id = str(payload["id"])
    title = notion_title(payload) or f"Notion page {page_id}"
    updated = payload.get("last_edited_time") if isinstance(payload.get("last_edited_time"), str) else None
    object_type = str(payload.get("object") or "page")
    summary = compact_sentence(f"Notion {object_type} {title} was last edited {updated or 'at an unknown time'}.")
    return ConnectorRecord(
        external_id=page_id,
        title=f"Notion: {title}",
        summary=summary,
        content_markdown="\n".join(
            ["## Notion summary", "", f"- Title: {title}", f"- Type: {object_type}", f"- Last edited: {updated or 'Unknown'}"]
        ),
        source_uri=payload.get("url") if isinstance(payload.get("url"), str) else None,
        item_timestamp=updated,
        tags=("connector", "notion", object_type),
        normalized={"pageId": page_id, "title": title, "lastEditedTime": updated, "object": object_type},
    )


def jira_record(payload: dict[str, Any], api_base_url: str) -> ConnectorRecord:
    issue_id = str(payload.get("id") or payload.get("key"))
    key = str(payload.get("key") or issue_id)
    fields = payload.get("fields") if isinstance(payload.get("fields"), dict) else {}
    title = str(fields.get("summary") or f"Jira issue {key}")
    status = nested_name(fields.get("status")) or "Unknown"
    updated = fields.get("updated") if isinstance(fields.get("updated"), str) else None
    summary = compact_sentence(f"Jira issue {key} is {status}: {title}")
    return ConnectorRecord(
        external_id=issue_id,
        title=f"Jira: {key} {title}",
        summary=summary,
        content_markdown="\n".join(["## Jira issue summary", "", f"- Issue: {key}", f"- Status: {status}", f"- Updated: {updated or 'Unknown'}", "", title]),
        source_uri=jira_issue_url(api_base_url, key),
        item_timestamp=updated,
        tags=("connector", "jira", "issue", status.lower().replace(" ", "-")),
        normalized={"issueId": issue_id, "key": key, "summary": title, "status": status, "updated": updated},
    )


def linear_record(payload: dict[str, Any]) -> ConnectorRecord:
    issue_id = str(payload["id"])
    identifier = str(payload.get("identifier") or issue_id)
    title = str(payload.get("title") or f"Linear issue {identifier}")
    status = nested_name(payload.get("state")) or "Unknown"
    assignee = nested_name(payload.get("assignee")) or "Unassigned"
    updated = payload.get("updatedAt") if isinstance(payload.get("updatedAt"), str) else None
    summary = compact_sentence(f"Linear issue {identifier} is {status}, assigned to {assignee}: {title}")
    return ConnectorRecord(
        external_id=issue_id,
        title=f"Linear: {identifier} {title}",
        summary=summary,
        content_markdown="\n".join(["## Linear issue summary", "", f"- Issue: {identifier}", f"- Status: {status}", f"- Assignee: {assignee}", f"- Updated: {updated or 'Unknown'}", "", title]),
        source_uri=payload.get("url") if isinstance(payload.get("url"), str) else None,
        item_timestamp=updated,
        tags=("connector", "linear", "issue", status.lower().replace(" ", "-")),
        normalized={"issueId": issue_id, "identifier": identifier, "title": title, "status": status, "assignee": assignee, "updatedAt": updated},
    )


def notion_title(payload: dict[str, Any]) -> str | None:
    properties = payload.get("properties")
    if not isinstance(properties, dict):
        return None
    for value in properties.values():
        if not isinstance(value, dict) or value.get("type") != "title":
            continue
        title_items = value.get("title")
        if not isinstance(title_items, list):
            continue
        parts = [
            item.get("plain_text")
            for item in title_items
            if isinstance(item, dict) and isinstance(item.get("plain_text"), str)
        ]
        title = " ".join(parts).strip()
        if title:
            return title
    return None


def nested_name(value: object) -> str | None:
    if isinstance(value, dict) and isinstance(value.get("name"), str):
        return value["name"]
    return None


def jira_issue_url(api_base_url: str, key: str) -> str | None:
    if "/rest/api/" not in api_base_url:
        return None
    return f"{api_base_url.split('/rest/api/', 1)[0]}/browse/{key}"


def headers_by_name(payload: dict[str, Any]) -> dict[str, str]:
    envelope = payload.get("payload")
    raw_headers = envelope.get("headers") if isinstance(envelope, dict) else []
    headers: dict[str, str] = {}
    for header in raw_headers if isinstance(raw_headers, list) else []:
        if not isinstance(header, dict):
            continue
        name = header.get("name")
        value = header.get("value")
        if isinstance(name, str) and isinstance(value, str):
            headers[name.lower()] = value
    return headers


def event_time(value: object) -> str | None:
    if not isinstance(value, dict):
        return None
    for key in ("dateTime", "date"):
        item = value.get(key)
        if isinstance(item, str):
            return item
    return None


def compact_sentence(value: str, limit: int = 220) -> str:
    normalized = " ".join(value.split())
    if len(normalized) <= limit:
        return normalized
    return normalized[: limit - 1].rstrip() + "."


class ConnectorScheduler:
    def next_sync_at(self, connector: ConnectorItem, *, from_time: datetime | None = None) -> str | None:
        if not connector.enabled or not connector.token_stored or connector.status in {"not_connected", "error"}:
            return None
        base = from_time or datetime.now(UTC)
        return (base + timedelta(minutes=connector.sync_interval_minutes)).isoformat().replace("+00:00", "Z")

    def is_due(self, connector: ConnectorItem, *, at_time: datetime | None = None) -> bool:
        if not connector.next_sync_at:
            return False
        due_at = datetime.fromisoformat(connector.next_sync_at.replace("Z", "+00:00"))
        return due_at <= (at_time or datetime.now(UTC))


class ConnectorManager:
    def __init__(self, data_dir: Path, privacy_firewall: PrivacyFirewall, memory_store: MemoryStore) -> None:
        self.data_dir = data_dir
        self.database_path = data_dir / "connectors.sqlite3"
        self.privacy_firewall = privacy_firewall
        self.memory_store = memory_store
        self.token_vault = TokenVault(data_dir, self.database_path)
        self.scheduler = ConnectorScheduler()
        self.registry = {
            definition.id: CONNECTOR_CLASSES[definition.id](definition)
            for definition in CONNECTOR_DEFINITIONS
        }

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.token_vault.initialize()
        with self.connect() as connection:
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS connectors (
                  id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  status TEXT NOT NULL,
                  enabled INTEGER NOT NULL,
                  scopes_json TEXT NOT NULL,
                  sync_interval_minutes INTEGER NOT NULL,
                  last_sync_at TEXT,
                  next_sync_at TEXT,
                  token_stored INTEGER NOT NULL,
                  token_updated_at TEXT,
                  last_error TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS connector_oauth_states (
                  state TEXT PRIMARY KEY,
                  connector_id TEXT NOT NULL,
                  redirect_uri TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  expires_at TEXT NOT NULL,
                  used_at TEXT
                );

                CREATE TABLE IF NOT EXISTS connector_sync_runs (
                  id TEXT PRIMARY KEY,
                  connector_id TEXT NOT NULL,
                  status TEXT NOT NULL,
                  reason TEXT NOT NULL,
                  started_at TEXT NOT NULL,
                  completed_at TEXT,
                  items_seen INTEGER NOT NULL DEFAULT 0,
                  items_written INTEGER NOT NULL DEFAULT 0,
                  error_message TEXT
                );

                CREATE TABLE IF NOT EXISTS connector_items (
                  id TEXT PRIMARY KEY,
                  connector_id TEXT NOT NULL,
                  external_id TEXT NOT NULL,
                  title TEXT NOT NULL,
                  summary TEXT NOT NULL,
                  source_uri TEXT,
                  item_timestamp TEXT,
                  normalized_json TEXT NOT NULL,
                  memory_id TEXT NOT NULL,
                  fetched_at TEXT NOT NULL,
                  UNIQUE(connector_id, external_id)
                );

                CREATE INDEX IF NOT EXISTS idx_connector_sync_runs_started_at
                ON connector_sync_runs(started_at);

                CREATE INDEX IF NOT EXISTS idx_connector_items_connector_id
                ON connector_items(connector_id);
                """
            )

        for connector in self.registry.values():
            self.register(connector)

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def register(self, connector: BaseConnector) -> ConnectorItem:
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                existing = connection.execute(
                    "SELECT created_at FROM connectors WHERE id = ?",
                    (connector.id,),
                ).fetchone()
                connection.execute(
                    """
                    INSERT INTO connectors (
                      id, name, status, enabled, scopes_json, sync_interval_minutes,
                      last_sync_at, next_sync_at, token_stored, token_updated_at,
                      last_error, created_at, updated_at
                    )
                    VALUES (?, ?, 'not_connected', 0, ?, ?, NULL, NULL, 0, NULL, NULL, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                      name = excluded.name,
                      scopes_json = excluded.scopes_json,
                      updated_at = excluded.updated_at
                    """,
                    (
                        connector.id,
                        connector.name,
                        json.dumps(connector.scopes),
                        connector.definition.default_sync_interval_minutes,
                        existing["created_at"] if existing else timestamp,
                        timestamp,
                    ),
                )
        return self.get_connector(connector.id)

    def list_connectors(self) -> ConnectorListResponse:
        self.initialize()
        with self.connect() as connection:
            rows = connection.execute("SELECT * FROM connectors").fetchall()
        by_id = {row["id"]: self._row_to_connector(row) for row in rows}
        return ConnectorListResponse(items=[by_id[connector_id] for connector_id in self.registry if connector_id in by_id])

    def get_connector(self, connector_id: str) -> ConnectorItem:
        with self.connect() as connection:
            row = connection.execute("SELECT * FROM connectors WHERE id = ?", (connector_id,)).fetchone()
        if not row:
            raise ConnectorNotFoundError(f"Unknown connector: {connector_id}")
        return self._row_to_connector(row)

    def update_settings(
        self,
        connector_id: str,
        *,
        enabled: bool | None = None,
        sync_interval_minutes: int | None = None,
    ) -> ConnectorItem:
        connector = self.get_connector(connector_id)
        next_enabled = connector.enabled if enabled is None else enabled
        next_interval = connector.sync_interval_minutes if sync_interval_minutes is None else sync_interval_minutes
        status: ConnectorStatus = connector.status
        if connector.token_stored:
            status = "connected" if next_enabled else "paused"
        next_sync_at = self.scheduler.next_sync_at(
            connector.model_copy(
                update={
                    "enabled": next_enabled,
                    "sync_interval_minutes": next_interval,
                    "status": status,
                }
            )
        )
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET enabled = ?, sync_interval_minutes = ?, status = ?,
                        next_sync_at = ?, last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (
                        1 if next_enabled else 0,
                        next_interval,
                        status,
                        next_sync_at,
                        timestamp,
                        connector_id,
                    ),
                )
        return self.get_connector(connector_id)

    def start_oauth(self, connector_id: str, redirect_uri: str | None = None) -> ConnectorOAuthStartResponse:
        connector = self._registered_connector(connector_id)
        redirect = redirect_uri or "deyana://oauth/callback"
        state = f"oauth_{uuid.uuid4().hex}"
        created_at = utc_timestamp()
        expires_at = (datetime.now(UTC) + timedelta(minutes=10)).isoformat().replace("+00:00", "Z")
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO connector_oauth_states (
                      state, connector_id, redirect_uri, created_at, expires_at, used_at
                    )
                    VALUES (?, ?, ?, ?, ?, NULL)
                    """,
                    (state, connector_id, redirect, created_at, expires_at),
                )
        return ConnectorOAuthStartResponse(
            connector=self.get_connector(connector_id),
            authorization_url=connector.build_authorization_url(state=state, redirect_uri=redirect),
            state=state,
            scopes=connector.scopes,
            redirect_uri=redirect,
            expires_at=expires_at,
            mock=not connector.oauth_configured(),
            oauth_configured=connector.oauth_configured(),
        )

    def complete_oauth(
        self,
        connector_id: str,
        *,
        state: str,
        code: str,
        user_approved: bool,
    ) -> tuple[ConnectorItem, PrivacyCheckResponse]:
        connector = self._registered_connector(connector_id)
        oauth_state = self._consume_oauth_state(connector_id, state)
        issued_at = utc_timestamp()
        http_client = ConnectorHttpClient(self.privacy_firewall, connector_id)
        if not connector.oauth_configured():
            privacy_result = self.privacy_firewall.check(
                PrivacyCheckRequest(
                    url=connector.token_url(),
                    method="POST",
                    purpose="oauth_api_fetch",
                    data_category="oauth_token",
                    payload_preview=f"{connector.name} mock OAuth token exchange",
                    user_approved=user_approved,
                    connector_id=connector_id,
                )
            )
            if not privacy_result.allowed:
                raise PrivacyPolicyError(privacy_result)
            token_payload = connector.create_mock_token(code=code, issued_at=issued_at)
        else:
            token_payload = connector.exchange_oauth_code(
                code=code,
                redirect_uri=oauth_state["redirect_uri"],
                issued_at=issued_at,
                http_client=http_client,
                user_approved=user_approved,
            )
            privacy_result = http_client.privacy_results[-1]
        token_updated_at = self.token_vault.store(connector_id, token_payload)
        current = self.get_connector(connector_id)
        next_sync_at = self.scheduler.next_sync_at(
            current.model_copy(update={"enabled": True, "token_stored": True, "status": "connected"})
        )
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connector_oauth_states
                    SET used_at = ?
                    WHERE state = ?
                    """,
                    (issued_at, oauth_state["state"]),
                )
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'connected', enabled = 1, token_stored = 1,
                        token_updated_at = ?, next_sync_at = ?, last_error = NULL,
                        updated_at = ?
                    WHERE id = ?
                    """,
                    (token_updated_at, next_sync_at, issued_at, connector_id),
                )
        return self.get_connector(connector_id), privacy_result

    def disconnect(self, connector_id: str) -> tuple[ConnectorItem, bool]:
        self._registered_connector(connector_id)
        token_deleted = self.token_vault.delete(connector_id)
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'not_connected', enabled = 0, token_stored = 0,
                        token_updated_at = NULL, last_sync_at = NULL, next_sync_at = NULL,
                        last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (timestamp, connector_id),
                )
        return self.get_connector(connector_id), token_deleted

    def start_sync(self, connector_id: str, *, reason: str = "manual") -> ConnectorSyncResponse:
        connector = self.get_connector(connector_id)
        if not connector.token_stored:
            run = self._create_sync_run(connector_id, reason=reason, status="failed")
            run = self._complete_sync_run(run.id, status="failed", error_message="Connector is not connected.")
            self._set_connector_error(connector_id, "Connector is not connected.")
            raise ConnectorStateError("Connector is not connected.")
        if not connector.enabled:
            run = self._create_sync_run(connector_id, reason=reason, status="skipped")
            run = self._complete_sync_run(run.id, status="skipped", error_message="Connector sync is paused.")
            return ConnectorSyncResponse(connector=connector, run=run)

        run = self._create_sync_run(connector_id, reason=reason, status="running")
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'syncing', last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (timestamp, connector_id),
                )
        return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run)

    def finish_sync(self, connector_id: str, run_id: str) -> tuple[ConnectorSyncResponse, PrivacyCheckResponse | None]:
        connector = self._registered_connector(connector_id)
        token = self.token_vault.read(connector_id)
        if not token:
            run = self._complete_sync_run(run_id, status="failed", error_message="Connector token is missing.")
            self._set_connector_error(connector_id, "Connector token is missing.")
            raise ConnectorStateError("Connector token is missing.")
        if not token.get("mock"):
            try:
                self.memory_store.require_vault_root()
            except ValueError as error:
                run = self._complete_sync_run(run_id, status="failed", error_message=str(error))
                self._set_connector_error(connector_id, str(error))
                return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run), None

        privacy_result = self.privacy_firewall.check(
            PrivacyCheckRequest(
                url=connector.api_probe_url(),
                method="GET",
                purpose="connector_api_fetch",
                data_category="connector_metadata",
                payload_preview=f"{connector.name} metadata sync",
                user_approved=True,
                connector_id=connector_id,
            )
        )
        if not privacy_result.allowed:
            run = self._complete_sync_run(run_id, status="failed", error_message=privacy_result.reason)
            self._set_connector_error(connector_id, privacy_result.reason)
            raise PrivacyPolicyError(privacy_result)

        try:
            http_client = ConnectorHttpClient(self.privacy_firewall, connector_id)
            result = connector.sync(ConnectorSyncContext(token=token, http_client=http_client))
            items_written = self._write_connector_records(connector_id, result.records)
        except (ConnectorError, ValueError) as error:
            run = self._complete_sync_run(run_id, status="failed", error_message=str(error))
            self._set_connector_error(connector_id, str(error))
            return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run), privacy_result

        run = self._complete_sync_run(
            run_id,
            status="completed",
            items_seen=result.items_seen,
            items_written=items_written,
        )
        current = self.get_connector(connector_id)
        completed_at = run.completed_at or utc_timestamp()
        next_sync_at = self.scheduler.next_sync_at(current.model_copy(update={"status": "connected"}))
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'connected', last_sync_at = ?, next_sync_at = ?,
                        last_error = NULL, updated_at = ?
                    WHERE id = ?
                    """,
                    (completed_at, next_sync_at, completed_at, connector_id),
                )
        return ConnectorSyncResponse(connector=self.get_connector(connector_id), run=run), privacy_result

    def list_sync_runs(self, *, limit: int = 20) -> ConnectorSyncRunsResponse:
        self.initialize()
        limit = max(1, min(limit, 100))
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM connector_sync_runs
                ORDER BY started_at DESC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
            total = connection.execute("SELECT COUNT(*) AS count FROM connector_sync_runs").fetchone()["count"]
        return ConnectorSyncRunsResponse(items=[self._row_to_run(row) for row in rows], total=total)

    def due_connectors(self) -> list[ConnectorItem]:
        return [connector for connector in self.list_connectors().items if self.scheduler.is_due(connector)]

    def _registered_connector(self, connector_id: str) -> BaseConnector:
        connector = self.registry.get(connector_id)
        if not connector:
            raise ConnectorNotFoundError(f"Unknown connector: {connector_id}")
        return connector

    def _consume_oauth_state(self, connector_id: str, state: str) -> sqlite3.Row:
        with self.connect() as connection:
            row = connection.execute(
                """
                SELECT * FROM connector_oauth_states
                WHERE state = ? AND connector_id = ?
                """,
                (state, connector_id),
            ).fetchone()
        if not row:
            raise ConnectorStateError("OAuth state is unknown.")
        if row["used_at"]:
            raise ConnectorStateError("OAuth state has already been used.")
        expires_at = datetime.fromisoformat(row["expires_at"].replace("Z", "+00:00"))
        if expires_at <= datetime.now(UTC):
            raise ConnectorStateError("OAuth state has expired.")
        return row

    def _create_sync_run(
        self,
        connector_id: str,
        *,
        reason: str,
        status: ConnectorSyncRunStatus,
    ) -> ConnectorSyncRun:
        run_id = f"sync_{uuid.uuid4().hex}"
        started_at = utc_timestamp()
        completed_at = started_at if status in {"completed", "failed", "skipped"} else None
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO connector_sync_runs (
                      id, connector_id, status, reason, started_at, completed_at,
                      items_seen, items_written, error_message
                    )
                    VALUES (?, ?, ?, ?, ?, ?, 0, 0, NULL)
                    """,
                    (run_id, connector_id, status, reason, started_at, completed_at),
                )
        return self._get_sync_run(run_id)

    def _complete_sync_run(
        self,
        run_id: str,
        *,
        status: ConnectorSyncRunStatus,
        items_seen: int = 0,
        items_written: int = 0,
        error_message: str | None = None,
    ) -> ConnectorSyncRun:
        completed_at = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connector_sync_runs
                    SET status = ?, completed_at = ?, items_seen = ?,
                        items_written = ?, error_message = ?
                    WHERE id = ?
                    """,
                    (status, completed_at, items_seen, items_written, error_message, run_id),
                )
        return self._get_sync_run(run_id)

    def _get_sync_run(self, run_id: str) -> ConnectorSyncRun:
        with self.connect() as connection:
            row = connection.execute("SELECT * FROM connector_sync_runs WHERE id = ?", (run_id,)).fetchone()
        if not row:
            raise ConnectorStateError("Sync run is missing.")
        return self._row_to_run(row)

    def _set_connector_error(self, connector_id: str, message: str) -> None:
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    UPDATE connectors
                    SET status = 'error', last_error = ?, updated_at = ?
                    WHERE id = ?
                    """,
                    (message, timestamp, connector_id),
                )

    def _write_connector_records(self, connector_id: str, records: tuple[ConnectorRecord, ...]) -> int:
        written = 0
        for record in records:
            if self._connector_record_exists(connector_id, record.external_id):
                continue

            memory = self.memory_store.create(
                MemoryCreateRequest(
                    type="connector_summary",
                    title=record.title,
                    summary=record.summary,
                    content_markdown=record.content_markdown,
                    source_type=connector_id,
                    source_id=record.external_id,
                    source_uri=record.source_uri,
                    importance=3,
                    tags=list(record.tags),
                )
            )
            self._insert_connector_record(connector_id, record, memory.id)
            written += 1
        return written

    def _connector_record_exists(self, connector_id: str, external_id: str) -> bool:
        with self.connect() as connection:
            row = connection.execute(
                """
                SELECT 1 FROM connector_items
                WHERE connector_id = ? AND external_id = ?
                """,
                (connector_id, external_id),
            ).fetchone()
        return row is not None

    def _insert_connector_record(self, connector_id: str, record: ConnectorRecord, memory_id: str) -> None:
        timestamp = utc_timestamp()
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO connector_items (
                      id, connector_id, external_id, title, summary, source_uri,
                      item_timestamp, normalized_json, memory_id, fetched_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        f"connector_item_{uuid.uuid4().hex}",
                        connector_id,
                        record.external_id,
                        record.title,
                        record.summary,
                        record.source_uri,
                        record.item_timestamp,
                        json.dumps(record.normalized, sort_keys=True),
                        memory_id,
                        timestamp,
                    ),
                )

    def _row_to_connector(self, row: sqlite3.Row) -> ConnectorItem:
        connector = self.registry.get(row["id"])
        return ConnectorItem(
            id=row["id"],
            name=row["name"],
            status=row["status"],
            enabled=bool(row["enabled"]),
            scopes=json.loads(row["scopes_json"]),
            oauth_configured=connector.oauth_configured() if connector else False,
            real_sync_supported=connector is not None,
            sync_interval_minutes=row["sync_interval_minutes"],
            last_sync_at=row["last_sync_at"],
            next_sync_at=row["next_sync_at"],
            token_stored=bool(row["token_stored"]),
            token_updated_at=row["token_updated_at"],
            last_error=row["last_error"],
            updated_at=row["updated_at"],
        )

    def _row_to_run(self, row: sqlite3.Row) -> ConnectorSyncRun:
        return ConnectorSyncRun(
            id=row["id"],
            connector_id=row["connector_id"],
            status=row["status"],
            reason=row["reason"],
            started_at=row["started_at"],
            completed_at=row["completed_at"],
            items_seen=row["items_seen"],
            items_written=row["items_written"],
            error_message=row["error_message"],
        )
