from __future__ import annotations

import html
import re
import subprocess
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote_plus
from urllib.request import Request, urlopen

from .memory import MemoryStore
from .models import (
    CodeTaskRequest,
    DayPlannerRequest,
    FileReadRequest,
    GitReadRequest,
    PrivacyCheckRequest,
    ToolListResponse,
    ToolManifest,
    ToolResultItem,
    ToolRunResponse,
    WebFetchRequest,
    WebSearchRequest,
)
from .privacy import PrivacyFirewall
from .runtime_time import utc_timestamp


TOOL_MANIFESTS = [
    ToolManifest(
        tool_id="web_search",
        name="Web search",
        description="Search the public web with only the user's explicit public query.",
        risk="public_web",
        requires_approval=True,
    ),
    ToolManifest(
        tool_id="fetch_page",
        name="Fetch page",
        description="Fetch and summarize a public webpage without sending private memory.",
        risk="public_web",
        requires_approval=True,
    ),
    ToolManifest(
        tool_id="read_file",
        name="Read file",
        description="Read a file only when it is inside an approved folder root.",
        risk="local_file",
        requires_approval=True,
    ),
    ToolManifest(
        tool_id="git_status",
        name="Git status",
        description="Read git status for an approved repository path.",
        risk="source_code",
        requires_approval=True,
    ),
    ToolManifest(
        tool_id="git_diff",
        name="Git diff",
        description="Read git diff for an approved repository path.",
        risk="source_code",
        requires_approval=True,
    ),
    ToolManifest(
        tool_id="commit_message",
        name="Commit message",
        description="Suggest a commit message from local git status and diff without committing.",
        risk="source_code",
        requires_approval=True,
        applies_changes=False,
    ),
    ToolManifest(
        tool_id="code_task",
        name="Code task",
        description="Explain code and propose changes before any edits are applied.",
        risk="source_code",
        requires_approval=True,
        applies_changes=False,
    ),
    ToolManifest(
        tool_id="day_planner",
        name="Day planner",
        description="Create a local day plan from user-provided commitments and local action items.",
        risk="low",
        requires_approval=False,
    ),
]


class ToolPermissionError(RuntimeError):
    def __init__(self, tool_id: str, message: str) -> None:
        super().__init__(message)
        self.tool_id = tool_id


class ToolExecutionError(RuntimeError):
    pass


class ToolService:
    def __init__(self, privacy_firewall: PrivacyFirewall, memory_store: MemoryStore) -> None:
        self.privacy_firewall = privacy_firewall
        self.memory_store = memory_store

    def list_tools(self) -> ToolListResponse:
        return ToolListResponse(tools=TOOL_MANIFESTS)

    def web_search(self, request: WebSearchRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("web_search", "Public web search requires approval.")
        query = normalize_space(request.query)
        if not query:
            raise ToolExecutionError("Search query is required.")
        url = f"https://duckduckgo.com/html/?q={quote_plus(query)}"
        privacy = self.privacy_firewall.guard(
            PrivacyCheckRequest(
                url=url,
                method="GET",
                purpose="public_web_fetch",
                data_category="public_query",
                payload_preview=query,
                user_approved=True,
            )
        )
        html_text = fetch_text(url, accept="text/html")
        items = parse_duckduckgo_results(html_text, limit=request.limit)
        summary = f"Found {len(items)} public results for: {query}"
        return ToolRunResponse(
            tool_id="web_search",
            status="completed",
            title="Web search results",
            summary=summary,
            content="\n".join(f"- {item.title}: {item.url}" for item in items) or "No results found.",
            items=items,
            privacy=privacy,
        )

    def fetch_page(self, request: WebFetchRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("fetch_page", "Public webpage fetch requires approval.")
        privacy = self.privacy_firewall.guard(
            PrivacyCheckRequest(
                url=request.url,
                method="GET",
                purpose="public_web_fetch",
                data_category="public_content",
                payload_preview="Public webpage fetch",
                user_approved=True,
            )
        )
        raw = fetch_text(request.url, accept="text/html,text/plain")
        text = html_to_text(raw)
        content = truncate(text, request.max_characters)
        return ToolRunResponse(
            tool_id="fetch_page",
            status="completed",
            title=first_line(content) or "Fetched webpage",
            summary=compact_sentence(content, 280) or "Fetched public webpage content.",
            content=content,
            privacy=privacy,
        )

    def read_file(self, request: FileReadRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("read_file", "Reading a local file requires folder approval.")
        file_path = resolve_inside_root(request.path, request.allowed_root)
        if not file_path.is_file():
            raise ToolExecutionError("Approved path is not a readable file.")
        content = truncate(file_path.read_text(encoding="utf-8", errors="replace"), request.max_characters)
        return ToolRunResponse(
            tool_id="read_file",
            status="completed",
            title=file_path.name,
            summary=f"Read {len(content)} characters from an approved local file.",
            content=content,
        )

    def git_status(self, request: GitReadRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("git_status", "Reading git status requires repository approval.")
        repo = require_git_repo(request.repo_path)
        output = run_git(repo, ["status", "--short"], request.max_characters)
        return ToolRunResponse(
            tool_id="git_status",
            status="completed",
            title="Git status",
            summary=git_status_summary(output),
            content=output or "Working tree clean.",
        )

    def git_diff(self, request: GitReadRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("git_diff", "Reading git diff requires repository approval.")
        repo = require_git_repo(request.repo_path)
        output = run_git(repo, ["diff", "--", "."], request.max_characters)
        return ToolRunResponse(
            tool_id="git_diff",
            status="completed",
            title="Git diff summary",
            summary=diff_summary(output),
            content=output or "No unstaged diff.",
        )

    def commit_message(self, request: GitReadRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("commit_message", "Commit message suggestion requires repository approval.")
        repo = require_git_repo(request.repo_path)
        status = run_git(repo, ["status", "--short"], request.max_characters)
        diff = run_git(repo, ["diff", "--stat"], request.max_characters)
        message = suggest_commit_message(status=status, diff_stat=diff)
        content = f"{message}\n\nRationale:\n{diff or status or 'No local changes detected.'}"
        return ToolRunResponse(
            tool_id="commit_message",
            status="completed",
            title="Suggested commit message",
            summary=message,
            content=content,
            applies_changes=False,
        )

    def code_task(self, request: CodeTaskRequest) -> ToolRunResponse:
        if not request.user_approved:
            return permission_required("code_task", "Coding explanation/proposal requires approval for source context.")
        goal = normalize_space(request.goal)
        context = truncate(request.context, 8000)
        proposal = build_code_proposal(goal=goal, context=context)
        return ToolRunResponse(
            tool_id="code_task",
            status="completed",
            title="Coding proposal",
            summary="Generated a local proposal only; no files were changed.",
            content=proposal,
            applies_changes=False,
        )

    def day_planner(self, request: DayPlannerRequest) -> ToolRunResponse:
        date = request.date or utc_timestamp()[:10]
        actions = self.memory_store.list_insights(insight_type="action_item", status="open", limit=12).items
        lines = [f"# Day plan - {date}", "", "## Focus blocks", ""]
        focus = [normalize_space(item) for item in request.focus if normalize_space(item)]
        if not focus:
            focus = ["Review priority work", "Clear open action items", "Plan tomorrow"]
        for index, item in enumerate(focus[:5], start=1):
            lines.append(f"{index}. {item}")
        commitments = [normalize_space(item) for item in request.commitments if normalize_space(item)]
        if commitments:
            lines.extend(["", "## Commitments", ""])
            lines.extend(f"- {item}" for item in commitments[:10])
        if actions:
            lines.extend(["", "## Local action items", ""])
            for action in actions[:8]:
                due = f" Due: {action.due_at}." if action.due_at else ""
                lines.append(f"- {action.title}{due}")
        if request.notes.strip():
            lines.extend(["", "## Notes", "", request.notes.strip()])
        return ToolRunResponse(
            tool_id="day_planner",
            status="completed",
            title=f"Day plan - {date}",
            summary=f"Local day plan with {len(focus)} focus blocks and {len(actions)} memory action items.",
            content="\n".join(lines),
        )


def permission_required(tool_id: str, message: str) -> ToolRunResponse:
    return ToolRunResponse(
        tool_id=tool_id,  # type: ignore[arg-type]
        status="permission_required",
        title="Permission required",
        summary=message,
        content=message,
        permission_required=True,
    )


def fetch_text(url: str, *, accept: str) -> str:
    request = Request(url, headers={"user-agent": "DEYANA-local-tool/0.1", "accept": accept}, method="GET")
    try:
        with urlopen(request, timeout=20) as response:
            content_type = response.headers.get("content-type", "")
            charset = "utf-8"
            match = re.search(r"charset=([^;\s]+)", content_type, re.IGNORECASE)
            if match:
                charset = match.group(1)
            return response.read().decode(charset, errors="replace")
    except HTTPError as error:
        raise ToolExecutionError(f"Tool request failed with HTTP {error.code}.") from error
    except URLError as error:
        raise ToolExecutionError(f"Tool request failed: {error.reason}") from error


def parse_duckduckgo_results(content: str, *, limit: int) -> list[ToolResultItem]:
    results: list[ToolResultItem] = []
    pattern = re.compile(
        r'<a[^>]+class="result__a"[^>]+href="(?P<url>[^"]+)"[^>]*>(?P<title>.*?)</a>',
        re.IGNORECASE | re.DOTALL,
    )
    for match in pattern.finditer(content):
        title = html_to_text(match.group("title"))
        url = html.unescape(match.group("url"))
        if title and url:
            results.append(ToolResultItem(title=title, summary=title, url=url, source="duckduckgo"))
        if len(results) >= limit:
            break
    return results


def html_to_text(content: str) -> str:
    text = re.sub(r"<(script|style).*?</\1>", " ", content, flags=re.IGNORECASE | re.DOTALL)
    text = re.sub(r"<br\s*/?>", "\n", text, flags=re.IGNORECASE)
    text = re.sub(r"</(p|div|li|h[1-6])>", "\n", text, flags=re.IGNORECASE)
    text = re.sub(r"<[^>]+>", " ", text)
    return normalize_space(html.unescape(text))


def resolve_inside_root(path: str, allowed_root: str) -> Path:
    root = Path(allowed_root).expanduser().resolve()
    target = Path(path).expanduser().resolve()
    try:
        target.relative_to(root)
    except ValueError as error:
        raise ToolExecutionError("File is outside the approved folder root.") from error
    return target


def require_git_repo(repo_path: str) -> Path:
    repo = Path(repo_path).expanduser().resolve()
    if not (repo / ".git").exists():
        raise ToolExecutionError("Approved path is not a git repository.")
    return repo


def run_git(repo: Path, args: list[str], max_characters: int) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        timeout=20,
        check=False,
    )
    output = result.stdout.strip() or result.stderr.strip()
    if result.returncode != 0:
        raise ToolExecutionError(output or "Git command failed.")
    return truncate(output, max_characters)


def git_status_summary(output: str) -> str:
    if not output.strip():
        return "Working tree clean."
    lines = [line for line in output.splitlines() if line.strip()]
    return f"{len(lines)} changed paths in working tree."


def diff_summary(output: str) -> str:
    if not output.strip():
        return "No unstaged diff."
    additions = sum(1 for line in output.splitlines() if line.startswith("+") and not line.startswith("+++"))
    deletions = sum(1 for line in output.splitlines() if line.startswith("-") and not line.startswith("---"))
    return f"Unstaged diff has about {additions} additions and {deletions} deletions."


def suggest_commit_message(*, status: str, diff_stat: str) -> str:
    text = f"{status}\n{diff_stat}".lower()
    if "connector" in text:
        return "feat(connectors): expand local connector sync support"
    if "tool" in text:
        return "feat(tools): add permissioned local assistant tools"
    if "memory" in text:
        return "feat(memory): improve local memory workflow"
    if status.strip():
        return "chore: update local assistant implementation"
    return "chore: no local changes detected"


def build_code_proposal(*, goal: str, context: str) -> str:
    lines = [
        "# Coding proposal",
        "",
        f"Goal: {goal or 'Explain or improve the provided code.'}",
        "",
        "## Explanation",
        "",
        compact_sentence(context, 900) if context else "No source context was provided.",
        "",
        "## Proposed change plan",
        "",
        "1. Identify the smallest production path that satisfies the goal.",
        "2. Keep business logic outside UI code and preserve existing architecture boundaries.",
        "3. Update tests/docs beside the implementation.",
        "4. Apply edits only after explicit user confirmation.",
    ]
    return "\n".join(lines)


def first_line(content: str) -> str:
    for line in content.splitlines():
        if line.strip():
            return compact_sentence(line, 120)
    return ""


def compact_sentence(value: str, limit: int = 220) -> str:
    normalized = normalize_space(value)
    if len(normalized) <= limit:
        return normalized
    return normalized[: limit - 1].rstrip(" ,.;:") + "."


def normalize_space(value: str) -> str:
    return " ".join(value.split()).strip()


def truncate(value: str, limit: int) -> str:
    if len(value) <= limit:
        return value
    return value[:limit].rstrip() + "\n\n[truncated]"
