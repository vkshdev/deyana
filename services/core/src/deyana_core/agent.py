from __future__ import annotations

import re
from dataclasses import dataclass

from .chat import ChatStore
from .identity import ASSISTANT_IDENTITY
from .local_models import ModelRouter
from .memory import MemoryStore
from .models import (
    ChatRoute,
    ChatMessageResponse,
    ChatRetrievalSummary,
    MemoryItem,
    MemorySourceReference,
    WebFetchRequest,
    WebSearchRequest,
    WebSourceReference,
)
from .privacy import PrivacyPolicyError
from .tools import ToolExecutionError, ToolService

MAX_SOURCES = 4
MAX_SOURCE_CHARS = 720
MAX_WEB_CONTEXT_CHARS = 7000
MAX_WEB_SOURCES = 5
URL_PATTERN = re.compile(r"https?://[^\s<>\"']+", re.IGNORECASE)
MEMORY_INTENT_PATTERN = re.compile(
    r"\b(?:remember|memory|vault|we discussed|did we decide|did we agree|"
    r"my (?:tasks?|notes?|meetings?|projects?|emails?|decisions?|schedule)|"
    r"our (?:roadmap|project|decision|plan)|the (?:roadmap|project note))\b",
    re.IGNORECASE,
)
WEB_INTENT_PATTERN = re.compile(
    r"\b(?:search (?:the )?(?:web|internet|online)|browse (?:the )?(?:web|internet)|"
    r"look ?up|find online|latest|current|today|news|live|recent|price|weather|"
    r"score|release|version|research|compare|who|when|where|why|how|what)\b",
    re.IGNORECASE,
)
CONVERSATION_PATTERN = re.compile(
    r"^(?:hi|hello|hey|thanks|thank you|good (?:morning|afternoon|evening)|"
    r"who are you|what can you do)[.!?\s]*$",
    re.IGNORECASE,
)
SCREEN_INTENT_PATTERN = re.compile(
    r"\b(?:what is on my screen|what am i looking at|read my screen|describe my screen|what's on my screen|what am i seeing|screenshot)\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class RetrievedMemory:
    item: MemoryItem
    snippet: str
    score: float


@dataclass(frozen=True)
class BuiltContext:
    prompt: str
    references: list[MemorySourceReference]
    web_references: list[WebSourceReference]
    compressed_characters: int
    token_estimate: int


@dataclass(frozen=True)
class WebContext:
    content: str
    references: list[WebSourceReference]
    error: str | None = None


class ChatIntentRouter:
    def route(self, content: str, *, use_memory: bool, allow_web: bool) -> ChatRoute:
        if SCREEN_INTENT_PATTERN.search(content):
            return "screen_query"
        if not allow_web:
            return "memory" if use_memory else "conversation"
        if extract_public_url(content):
            return "web_fetch"
        if use_memory and MEMORY_INTENT_PATTERN.search(content):
            return "memory"
        if CONVERSATION_PATTERN.fullmatch(content.strip()):
            return "conversation"
        if WEB_INTENT_PATTERN.search(content) or content.rstrip().endswith("?"):
            return "web_search"
        return "conversation"


class MemoryRetriever:
    def __init__(self, memory_store: MemoryStore) -> None:
        self.memory_store = memory_store

    def retrieve(self, query: str, limit: int = MAX_SOURCES) -> list[RetrievedMemory]:
        terms = tokenize(query)
        if not terms:
            return []

        candidates = self.memory_store.export().items
        ranked: list[RetrievedMemory] = []

        for item in candidates:
            searchable = weighted_text(item)
            score = score_text(searchable, terms)
            if score <= 0:
                continue
            snippet = compress_memory(item, terms, max_chars=MAX_SOURCE_CHARS)
            ranked.append(RetrievedMemory(item=item, snippet=snippet, score=score))

        ranked.sort(
            key=lambda result: (
                result.score,
                result.item.importance,
                result.item.updated_at,
            ),
            reverse=True,
        )
        return ranked[: max(1, min(limit, MAX_SOURCES))]


class ContextBuilder:
    def build(
        self,
        user_message: str,
        retrieved: list[RetrievedMemory],
        recent_history: list[str],
        route: ChatRoute,
        web_context: WebContext,
        max_context_chars: int,
    ) -> BuiltContext:
        references = [
            MemorySourceReference(
                id=result.item.id,
                title=result.item.title,
                label=f"S{index}",
                markdown_path=result.item.markdown_path,
                source_type=result.item.source_type,
                source_uri=result.item.source_uri,
                snippet=result.snippet,
                score=round(result.score, 3),
                updated_at=result.item.updated_at,
            )
            for index, result in enumerate(retrieved, start=1)
        ]
        memory_context = render_memory_context(references, max_context_chars)
        public_web_context = web_context.content.strip()
        history_context = render_history_context(recent_history)
        compressed_characters = len(memory_context) + len(public_web_context)
        web_status = (
            f"Live public web retrieval failed: {web_context.error}"
            if web_context.error
            else public_web_context or "No public web context was requested."
        )
        prompt = (
            f"You are {ASSISTANT_IDENTITY}, a local-first private desktop AI assistant.\n"
            "Use only the local memory context below when it is relevant. "
            "Do not invent memory that is not present. "
            "Cite memory claims inline with [S1], [S2], etc. "
            "For public web claims, use the public web context and cite [W1], [W2], etc. "
            "If live retrieval failed, say so clearly and do not claim that stale model knowledge is current. "
            "Never send or infer private memory as part of a public web query. "
            "Never suggest cloud AI services.\n\n"
            f"ROUTE: {route}\n\n"
            f"{history_context}"
            "LOCAL MEMORY CONTEXT:\n"
            f"{memory_context or 'No matching local memory was retrieved.'}\n\n"
            "PUBLIC WEB CONTEXT:\n"
            f"{web_status}\n\n"
            f"USER QUESTION:\n{user_message.strip()}\n\n"
            "ASSISTANT:"
        )
        return BuiltContext(
            prompt=prompt,
            references=references,
            web_references=web_context.references,
            compressed_characters=compressed_characters,
            token_estimate=estimate_tokens(prompt),
        )


class ChatAgent:
    def __init__(
        self,
        memory_store: MemoryStore,
        chat_store: ChatStore,
        model_router: ModelRouter,
        tool_service: ToolService | None = None,
    ) -> None:
        self.retriever = MemoryRetriever(memory_store)
        self.context_builder = ContextBuilder()
        self.chat_store = chat_store
        self.model_router = model_router
        self.tool_service = tool_service
        self.intent_router = ChatIntentRouter()

    def answer(
        self,
        content: str,
        *,
        use_memory: bool = True,
        allow_web: bool = False,
    ) -> ChatMessageResponse:
        clean_content = content.strip()
        if not clean_content:
            raise ValueError("Chat message cannot be empty.")

        route = self.intent_router.route(
            clean_content,
            use_memory=use_memory,
            allow_web=allow_web,
        )

        if route == "screen_query":
            from .vision import VisionService
            profile = self.model_router.store.read_settings().model_profile
            vision = VisionService(profile=profile)
            vision_response = vision.query_screen(clean_content)
            
            user_msg = self.chat_store.append("user", clean_content, vision.model)
            assistant_msg = self.chat_store.append("assistant", vision_response, vision.model)
            return ChatMessageResponse(
                user_message=user_msg,
                assistant_message=assistant_msg,
                model=vision.model,
                latency_ms=0,
                retrieval=ChatRetrievalSummary(
                    query=clean_content,
                    route=route,
                    retrieved=0,
                    compressed_characters=0,
                    context_tokens_estimate=0,
                ),
            )

        profile = self.model_router.store.read_settings().model_profile
        max_context_chars = 64000 if profile == "ultra" else (16000 if profile == "power" else 2600)
        num_predict_tokens = 2048 if profile == "ultra" else (1024 if profile == "power" else 640)

        retrieved = self.retriever.retrieve(clean_content) if route == "memory" else []
        web_context = self.retrieve_web_context(clean_content, route)
        recent_history = self.recent_history_lines()
        context = self.context_builder.build(
            clean_content,
            retrieved,
            recent_history,
            route,
            web_context,
            max_context_chars=max_context_chars,
        )
        generation = self.model_router.generate_prompt(
            context.prompt,
            temperature=0.22,
            num_predict=num_predict_tokens,
        )
        response_text = ensure_source_footer(
            generation.response,
            context.references,
            context.web_references,
        )
        user_message = self.chat_store.append("user", clean_content, generation.model)
        assistant_message = self.chat_store.append(
            "assistant",
            response_text,
            generation.model,
            source_references=context.references,
            web_source_references=context.web_references,
        )
        return ChatMessageResponse(
            user_message=user_message,
            assistant_message=assistant_message,
            model=generation.model,
            latency_ms=generation.latency_ms,
            sources=context.references,
            web_sources=context.web_references,
            retrieval=ChatRetrievalSummary(
                query=clean_content,
                route=route,
                retrieved=len(context.references),
                web_retrieved=len(context.web_references),
                compressed_characters=context.compressed_characters,
                context_tokens_estimate=context.token_estimate,
            ),
        )

    def retrieve_web_context(self, content: str, route: ChatRoute) -> WebContext:
        if route not in {"web_search", "web_fetch"}:
            return WebContext(content="", references=[])
        if not self.tool_service:
            return WebContext(
                content="",
                references=[],
                error="Public web tools are unavailable in this runtime.",
            )

        try:
            if route == "web_fetch":
                url = extract_public_url(content)
                if not url:
                    return WebContext(content="", references=[], error="No public URL was found.")
                result = self.tool_service.fetch_page(
                    WebFetchRequest(url=url, user_approved=True, max_characters=MAX_WEB_CONTEXT_CHARS)
                )
                reference = WebSourceReference(
                    title=result.title,
                    url=url,
                    snippet=result.summary,
                    source="public_webpage",
                )
                return WebContext(
                    content=render_web_page_context(reference, result.content),
                    references=[reference],
                )

            result = self.tool_service.web_search(
                WebSearchRequest(query=content, limit=MAX_WEB_SOURCES, user_approved=True)
            )
            references = [
                WebSourceReference(
                    title=item.title,
                    url=item.url,
                    snippet=item.summary,
                    source=item.source or "public_web",
                )
                for item in result.items
                if item.url
            ]
            if not references:
                return WebContext(
                    content="",
                    references=[],
                    error="Public web search returned no usable results.",
                )
            return WebContext(
                content=render_web_search_context(references),
                references=references,
            )
        except (PrivacyPolicyError, ToolExecutionError) as error:
            return WebContext(content="", references=[], error=str(error))

    def recent_history_lines(self, limit: int = 6) -> list[str]:
        messages = self.chat_store.history(limit=limit)
        lines: list[str] = []
        for message in messages:
            role = "User" if message.role == "user" else "Assistant"
            lines.append(f"{role}: {single_line(message.content)[:360]}")
        return lines


def weighted_text(item: MemoryItem) -> str:
    tags = " ".join(item.tags)
    return " ".join(
        [
            item.title,
            item.title,
            item.summary,
            item.summary,
            tags,
            item.content_markdown,
        ]
    ).lower()


def score_text(text: str, terms: list[str]) -> float:
    score = 0.0
    for term in terms:
        occurrences = text.count(term)
        if occurrences:
            score += 1.0 + min(occurrences, 4) * 0.35
    phrase = " ".join(terms)
    if len(terms) > 1 and phrase in text:
        score += 2.5
    return score


def compress_memory(item: MemoryItem, terms: list[str], max_chars: int) -> str:
    text = normalize_space(f"{item.summary}. {item.content_markdown}")
    if len(text) <= max_chars:
        return text

    sentences = split_sentences(text)
    ranked = sorted(
        sentences,
        key=lambda sentence: score_text(sentence.lower(), terms),
        reverse=True,
    )
    selected: list[str] = []
    total = 0
    for sentence in ranked:
        if total + len(sentence) + 1 > max_chars:
            continue
        selected.append(sentence)
        total += len(sentence) + 1
        if total >= max_chars * 0.72:
            break

    if not selected:
        return text[: max_chars - 1].rstrip() + "..."
    return normalize_space(" ".join(selected))[:max_chars]


def render_memory_context(references: list[MemorySourceReference], max_context_chars: int) -> str:
    lines: list[str] = []
    total = 0
    for reference in references:
        path = reference.markdown_path or reference.source_uri or reference.source_type
        block = (
            f"[{reference.label}] {reference.title}\n"
            f"Path: {path}\n"
            f"Updated: {reference.updated_at}\n"
            f"Compressed snippet: {reference.snippet}\n"
        )
        if total + len(block) > max_context_chars:
            break
        lines.append(block)
        total += len(block)
    return "\n".join(lines).strip()


def render_history_context(lines: list[str]) -> str:
    if not lines:
        return ""
    return "RECENT LOCAL CHAT HISTORY:\n" + "\n".join(lines[-6:]) + "\n\n"


def render_web_search_context(references: list[WebSourceReference]) -> str:
    if not references:
        return "The public web search completed but returned no usable results."
    return "\n".join(
        (
            f"[W{index}] {reference.title}\n"
            f"URL: {reference.url}\n"
            f"Search extract: {reference.snippet}"
        )
        for index, reference in enumerate(references, start=1)
    )[:MAX_WEB_CONTEXT_CHARS]


def render_web_page_context(reference: WebSourceReference, content: str) -> str:
    return (
        f"[W1] {reference.title}\n"
        f"URL: {reference.url}\n"
        f"Fetched page content:\n{content}"
    )[:MAX_WEB_CONTEXT_CHARS]


def ensure_source_footer(
    response: str,
    references: list[MemorySourceReference],
    web_references: list[WebSourceReference],
) -> str:
    clean_response = response.strip()
    footers: list[str] = []

    memory_labels = [f"[{reference.label}]" for reference in references]
    if references and not any(label in clean_response for label in memory_labels):
        source_line = ", ".join(
            f"[{reference.label}] {reference.title}" for reference in references
        )
        footers.append(f"Sources: {source_line}")

    web_labels = [f"[W{index}]" for index in range(1, len(web_references) + 1)]
    if web_references and not any(label in clean_response for label in web_labels):
        source_line = ", ".join(
            f"[W{index}] {reference.title} - {reference.url}"
            for index, reference in enumerate(web_references, start=1)
        )
        footers.append(f"Web sources: {source_line}")

    if not footers:
        return clean_response
    return f"{clean_response}\n\n" + "\n".join(footers)


def extract_public_url(value: str) -> str | None:
    match = URL_PATTERN.search(value)
    if not match:
        return None
    return match.group(0).rstrip(".,;:!?)]}")


def tokenize(value: str) -> list[str]:
    stop_words = {
        "about",
        "after",
        "again",
        "from",
        "have",
        "into",
        "tell",
        "that",
        "the",
        "this",
        "what",
        "when",
        "where",
        "with",
        "your",
    }
    terms = re.findall(r"[a-zA-Z0-9][a-zA-Z0-9_-]{2,}", value.lower())
    unique: list[str] = []
    for term in terms:
        if term in stop_words or term in unique:
            continue
        unique.append(term)
    return unique[:12]


def split_sentences(value: str) -> list[str]:
    sentences = re.split(r"(?<=[.!?])\s+", value)
    return [sentence.strip() for sentence in sentences if sentence.strip()]


def normalize_space(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def single_line(value: str) -> str:
    return normalize_space(value.replace("\n", " "))


def estimate_tokens(value: str) -> int:
    return max(1, len(value.split()))
