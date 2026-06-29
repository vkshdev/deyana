from __future__ import annotations

import re
from dataclasses import dataclass

from .chat import ChatStore
from .identity import ASSISTANT_IDENTITY
from .local_models import ModelRouter
from .memory import MemoryStore
from .models import (
    ChatMessageResponse,
    ChatRetrievalSummary,
    MemoryItem,
    MemorySourceReference,
)

MAX_SOURCES = 4
MAX_SOURCE_CHARS = 720
MAX_CONTEXT_CHARS = 2600


@dataclass(frozen=True)
class RetrievedMemory:
    item: MemoryItem
    snippet: str
    score: float


@dataclass(frozen=True)
class BuiltContext:
    prompt: str
    references: list[MemorySourceReference]
    compressed_characters: int
    token_estimate: int


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
        memory_context = render_memory_context(references)
        history_context = render_history_context(recent_history)
        compressed_characters = len(memory_context)
        prompt = (
            f"You are {ASSISTANT_IDENTITY}, a local-first private desktop AI assistant.\n"
            "Use only the local memory context below when it is relevant. "
            "Do not invent memory that is not present. "
            "Cite memory claims inline with [S1], [S2], etc. "
            "If the context is empty or not relevant, say what is missing briefly. "
            "Never suggest cloud AI services.\n\n"
            f"{history_context}"
            "LOCAL MEMORY CONTEXT:\n"
            f"{memory_context or 'No matching local memory was retrieved.'}\n\n"
            f"USER QUESTION:\n{user_message.strip()}\n\n"
            "ASSISTANT:"
        )
        return BuiltContext(
            prompt=prompt,
            references=references,
            compressed_characters=compressed_characters,
            token_estimate=estimate_tokens(prompt),
        )


class ChatAgent:
    def __init__(
        self,
        memory_store: MemoryStore,
        chat_store: ChatStore,
        model_router: ModelRouter,
    ) -> None:
        self.retriever = MemoryRetriever(memory_store)
        self.context_builder = ContextBuilder()
        self.chat_store = chat_store
        self.model_router = model_router

    def answer(self, content: str, *, use_memory: bool = True) -> ChatMessageResponse:
        clean_content = content.strip()
        if not clean_content:
            raise ValueError("Chat message cannot be empty.")

        retrieved = self.retriever.retrieve(clean_content) if use_memory else []
        recent_history = self.recent_history_lines()
        context = self.context_builder.build(clean_content, retrieved, recent_history)
        generation = self.model_router.generate_prompt(
            context.prompt,
            temperature=0.22,
            num_predict=640,
        )
        response_text = ensure_source_footer(generation.response, context.references)
        user_message = self.chat_store.append("user", clean_content, generation.model)
        assistant_message = self.chat_store.append(
            "assistant",
            response_text,
            generation.model,
            source_references=context.references,
        )
        return ChatMessageResponse(
            user_message=user_message,
            assistant_message=assistant_message,
            model=generation.model,
            latency_ms=generation.latency_ms,
            sources=context.references,
            retrieval=ChatRetrievalSummary(
                query=clean_content,
                retrieved=len(context.references),
                compressed_characters=context.compressed_characters,
                context_tokens_estimate=context.token_estimate,
            ),
        )

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


def render_memory_context(references: list[MemorySourceReference]) -> str:
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
        if total + len(block) > MAX_CONTEXT_CHARS:
            break
        lines.append(block)
        total += len(block)
    return "\n".join(lines).strip()


def render_history_context(lines: list[str]) -> str:
    if not lines:
        return ""
    return "RECENT LOCAL CHAT HISTORY:\n" + "\n".join(lines[-6:]) + "\n\n"


def ensure_source_footer(response: str, references: list[MemorySourceReference]) -> str:
    clean_response = response.strip()
    if not references:
        return clean_response

    labels = [f"[{reference.label}]" for reference in references]
    if any(label in clean_response for label in labels):
        return clean_response

    source_line = ", ".join(
        f"[{reference.label}] {reference.title}" for reference in references
    )
    return f"{clean_response}\n\nSources: {source_line}"


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
