from __future__ import annotations

import re
from dataclasses import dataclass


STOPWORDS = {
    "about",
    "after",
    "again",
    "also",
    "and",
    "are",
    "because",
    "before",
    "but",
    "can",
    "connector",
    "data",
    "from",
    "has",
    "have",
    "into",
    "local",
    "memory",
    "more",
    "not",
    "now",
    "our",
    "should",
    "summary",
    "that",
    "the",
    "this",
    "through",
    "with",
    "would",
}

ACTION_PATTERNS = re.compile(
    r"\b(todo|action|follow up|follow-up|need to|needs to|must|should|next step|remind|deadline|due)\b",
    re.IGNORECASE,
)
DECISION_PATTERNS = re.compile(
    r"\b(decided|decision|approved|rejected|agreed|choose|chose|selected|go with|use .+ instead|will use)\b",
    re.IGNORECASE,
)
URGENT_PATTERNS = re.compile(r"\b(urgent|blocked|deadline|asap|critical|important|launch|security)\b", re.IGNORECASE)
EMAIL_PATTERN = re.compile(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", re.IGNORECASE)
URL_PATTERN = re.compile(r"https?://[^\s)>\]]+")
REPO_PATTERN = re.compile(r"\b[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\b")
DATE_PATTERN = re.compile(
    r"\b(?:\d{4}-\d{2}-\d{2}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2}(?:,\s*\d{4})?)\b",
    re.IGNORECASE,
)
CAPITALIZED_ENTITY_PATTERN = re.compile(r"\b(?:[A-Z][a-zA-Z0-9'&.-]+(?:\s+|$)){2,4}")
EXTRACTION_SECTION_HEADINGS = {
    "## extracted action items",
    "## extracted decisions",
    "## extracted entities",
}


@dataclass(frozen=True)
class EntityCandidate:
    name: str
    entity_type: str
    source_text: str


@dataclass(frozen=True)
class InsightCandidate:
    insight_type: str
    title: str
    detail: str
    due_at: str | None = None


@dataclass(frozen=True)
class MemoryAnalysis:
    summary: str
    tags: tuple[str, ...]
    importance: int
    entities: tuple[EntityCandidate, ...]
    action_items: tuple[InsightCandidate, ...]
    decisions: tuple[InsightCandidate, ...]
    content_markdown: str


def analyze_memory(
    *,
    title: str,
    summary: str,
    content_markdown: str | None,
    source_type: str,
    memory_type: str,
    existing_tags: list[str],
    existing_importance: int,
) -> MemoryAnalysis:
    body = strip_generated_extraction_sections((content_markdown or summary or title).strip())
    text_source = body or summary or title
    text = normalize_text(f"{title}\n{summary}\n{strip_markdown(text_source)}")
    sentences = split_sentences(text)
    useful_summary = choose_summary(summary=summary, title=title, sentences=sentences)
    entities = extract_entities(text)
    if memory_type in {"daily_summary", "project_summary"}:
        action_items: tuple[InsightCandidate, ...] = ()
        decisions: tuple[InsightCandidate, ...] = ()
    else:
        action_items = extract_action_items(sentences)
        decisions = extract_decisions(sentences)
    tags = derive_tags(
        text=text,
        source_type=source_type,
        memory_type=memory_type,
        existing_tags=existing_tags,
        entities=entities,
        action_items=action_items,
        decisions=decisions,
    )
    importance = score_importance(
        text=text,
        existing_importance=existing_importance,
        action_items=action_items,
        decisions=decisions,
        source_type=source_type,
    )
    enriched_content = append_extraction_sections(
        body or default_content(title, useful_summary),
        entities=entities,
        action_items=action_items,
        decisions=decisions,
    )
    return MemoryAnalysis(
        summary=useful_summary,
        tags=tags,
        importance=importance,
        entities=entities,
        action_items=action_items,
        decisions=decisions,
        content_markdown=enriched_content,
    )


def choose_summary(*, summary: str, title: str, sentences: list[str]) -> str:
    value = normalize_text(summary)
    if 8 <= len(value) <= 260:
        return value
    if value and len(value) < 24 and sentences:
        return compact_sentence(f"{value}. {sentences[0]}", 260)
    if sentences:
        return compact_sentence(" ".join(sentences[:2]), 260)
    return compact_sentence(title, 260)


def extract_entities(text: str) -> tuple[EntityCandidate, ...]:
    candidates: list[EntityCandidate] = []
    for value in EMAIL_PATTERN.findall(text):
        candidates.append(EntityCandidate(name=value, entity_type="email", source_text=value))
    for value in URL_PATTERN.findall(text):
        candidates.append(EntityCandidate(name=value.rstrip(".,;"), entity_type="url", source_text=value))
    for value in REPO_PATTERN.findall(text):
        if "://" not in value and "@" not in value:
            candidates.append(EntityCandidate(name=value, entity_type="repository", source_text=value))
    for value in DATE_PATTERN.findall(text):
        candidates.append(EntityCandidate(name=value, entity_type="date", source_text=value))
    for match in CAPITALIZED_ENTITY_PATTERN.finditer(text):
        value = normalize_text(match.group(0))
        if value and not value.lower().startswith(("the ", "this ", "todo ")):
            candidates.append(EntityCandidate(name=value, entity_type=classify_named_entity(value), source_text=value))
    return unique_entities(candidates)


def extract_action_items(sentences: list[str]) -> tuple[InsightCandidate, ...]:
    insights: list[InsightCandidate] = []
    for sentence in sentences:
        if not ACTION_PATTERNS.search(sentence):
            continue
        insights.append(
            InsightCandidate(
                insight_type="action_item",
                title=compact_sentence(sentence, 84),
                detail=sentence,
                due_at=extract_due_date(sentence),
            )
        )
    return tuple(insights[:8])


def extract_decisions(sentences: list[str]) -> tuple[InsightCandidate, ...]:
    insights: list[InsightCandidate] = []
    for sentence in sentences:
        if not DECISION_PATTERNS.search(sentence):
            continue
        insights.append(
            InsightCandidate(
                insight_type="decision",
                title=compact_sentence(sentence, 84),
                detail=sentence,
            )
        )
    return tuple(insights[:8])


def derive_tags(
    *,
    text: str,
    source_type: str,
    memory_type: str,
    existing_tags: list[str],
    entities: tuple[EntityCandidate, ...],
    action_items: tuple[InsightCandidate, ...],
    decisions: tuple[InsightCandidate, ...],
) -> tuple[str, ...]:
    tags = {tagify(tag) for tag in existing_tags if tagify(tag)}
    tags.add(tagify(memory_type))
    tags.add(tagify(source_type))
    if action_items:
        tags.add("action-item")
    if decisions:
        tags.add("decision")
    for entity in entities:
        if entity.entity_type in {"repository", "organization", "project"}:
            tags.add(tagify(entity.name.split("/")[-1]))
    for token in keyword_candidates(text):
        tags.add(token)
        if len(tags) >= 12:
            break
    return tuple(sorted(tag for tag in tags if tag))


def score_importance(
    *,
    text: str,
    existing_importance: int,
    action_items: tuple[InsightCandidate, ...],
    decisions: tuple[InsightCandidate, ...],
    source_type: str,
) -> int:
    score = max(1, min(existing_importance or 3, 5))
    if action_items:
        score += 1
    if decisions:
        score += 1
    if URGENT_PATTERNS.search(text):
        score += 1
    if source_type in {"github", "calendar"}:
        score += 1
    return max(1, min(score, 5))


def append_extraction_sections(
    content: str,
    *,
    entities: tuple[EntityCandidate, ...],
    action_items: tuple[InsightCandidate, ...],
    decisions: tuple[InsightCandidate, ...],
) -> str:
    base = strip_generated_extraction_sections(content).strip()
    sections: list[str] = [base]
    if action_items:
        sections.extend(["", "## Extracted action items", ""])
        sections.extend(f"- {item.detail}" for item in action_items)
    if decisions:
        sections.extend(["", "## Extracted decisions", ""])
        sections.extend(f"- {item.detail}" for item in decisions)
    if entities:
        sections.extend(["", "## Extracted entities", ""])
        sections.extend(f"- {entity.entity_type}: {entity.name}" for entity in entities[:16])
    return "\n".join(sections).strip()


def strip_generated_extraction_sections(content: str) -> str:
    lines = content.splitlines()
    kept: list[str] = []
    skipping_generated_section = False

    for line in lines:
        normalized_heading = line.strip().lower()
        is_generated_heading = normalized_heading in EXTRACTION_SECTION_HEADINGS
        is_next_user_section = skipping_generated_section and normalized_heading.startswith("## ") and not is_generated_heading

        if is_next_user_section:
            skipping_generated_section = False

        if is_generated_heading:
            skipping_generated_section = True
            continue

        if skipping_generated_section:
            continue

        kept.append(line)

    return "\n".join(kept).strip()


def build_daily_summary(date: str, items: list[object]) -> tuple[str, str, str, list[str]]:
    title = f"Daily summary - {date}"
    lines = [f"## Daily summary for {date}", ""]
    tags = ["daily-summary", date]
    if not items:
        return title, "No memory items were recorded for this day.", "\n".join(lines + ["No local memory items found."]), tags
    source_counts: dict[str, int] = {}
    action_count = 0
    decision_count = 0
    for item in items:
        source = getattr(item, "source_type", "memory")
        source_counts[source] = source_counts.get(source, 0) + 1
        action_count += len(getattr(item, "action_items", []))
        decision_count += len(getattr(item, "decisions", []))
    top_sources = ", ".join(f"{source} ({count})" for source, count in sorted(source_counts.items()))
    summary = (
        f"{len(items)} local memory items recorded. Sources: {top_sources}. "
        f"Action items: {action_count}. Decisions: {decision_count}."
    )
    lines.extend(["### Highlights", ""])
    for item in items[:12]:
        lines.append(f"- **{getattr(item, 'title')}** ({source_label(item)}): {getattr(item, 'summary')}")
    append_insight_rollup_sections(lines, items)
    return title, summary, "\n".join(lines), tags


def build_project_summary(project: str, items: list[object]) -> tuple[str, str, str, list[str]]:
    normalized_project = normalize_text(project)
    title = f"Project summary - {normalized_project}"
    tags = ["project-summary", tagify(normalized_project)]
    if not items:
        return title, f"No local memory matched {normalized_project}.", f"## Project summary\n\nNo local memory matched {normalized_project}.", tags
    action_count = sum(len(getattr(item, "action_items", [])) for item in items)
    decision_count = sum(len(getattr(item, "decisions", [])) for item in items)
    summary = (
        f"{normalized_project} has {len(items)} related memory items, "
        f"{action_count} action items, and {decision_count} decisions."
    )
    lines = [f"## Project summary: {normalized_project}", "", "### Related memory", ""]
    for item in items[:16]:
        lines.append(f"- **{getattr(item, 'title')}** ({source_label(item)}): {getattr(item, 'summary')}")
    append_insight_rollup_sections(lines, items)
    return title, summary, "\n".join(lines), tags


def append_insight_rollup_sections(lines: list[str], items: list[object]) -> None:
    action_items = collect_item_insights(items, "action_items")
    decisions = collect_item_insights(items, "decisions")

    if action_items:
        lines.extend(["", "### Open action items", ""])
        for item, insight in action_items[:12]:
            due = getattr(insight, "due_at", None)
            due_text = f" Due: {due}." if due else ""
            lines.append(
                f"- **{getattr(item, 'title')}** ({source_label(item)}): {getattr(insight, 'detail')}{due_text}"
            )

    if decisions:
        lines.extend(["", "### Decisions", ""])
        for item, insight in decisions[:12]:
            lines.append(f"- **{getattr(item, 'title')}** ({source_label(item)}): {getattr(insight, 'detail')}")


def collect_item_insights(items: list[object], field_name: str) -> list[tuple[object, object]]:
    collected: list[tuple[object, object]] = []
    for item in items:
        for insight in getattr(item, field_name, []):
            collected.append((item, insight))
    return collected


def source_label(item: object) -> str:
    source_type = getattr(item, "source_type", "memory")
    source_id = getattr(item, "source_id", None)
    if source_id:
        return f"{source_type}:{source_id}"
    return source_type


def split_sentences(text: str) -> list[str]:
    parts = re.split(r"(?<=[.!?])\s+|\n+", text)
    return [compact_sentence(part.strip(), 320) for part in parts if len(part.strip()) > 8][:20]


def strip_markdown(value: str) -> str:
    text = re.sub(r"```.*?```", " ", value, flags=re.DOTALL)
    text = re.sub(r"`([^`]+)`", r"\1", text)
    text = re.sub(r"!\[[^\]]*\]\([^)]+\)", " ", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = re.sub(r"^[#>*\-\s]+", "", text, flags=re.MULTILINE)
    return text


def normalize_text(value: str) -> str:
    return " ".join(value.split()).strip()


def compact_sentence(value: str, limit: int) -> str:
    normalized = normalize_text(value)
    if len(normalized) <= limit:
        return normalized
    return normalized[: limit - 1].rstrip(" ,.;:") + "."


def tagify(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")[:40]


def keyword_candidates(text: str) -> list[str]:
    words = re.findall(r"\b[a-zA-Z][a-zA-Z0-9-]{3,}\b", text.lower())
    ranked: dict[str, int] = {}
    for word in words:
        if word in STOPWORDS:
            continue
        ranked[word] = ranked.get(word, 0) + 1
    return [word for word, _count in sorted(ranked.items(), key=lambda item: (-item[1], item[0]))[:10]]


def classify_named_entity(value: str) -> str:
    lowered = value.lower()
    if any(token in lowered for token in ["inc", "llc", "labs", "studio", "company"]):
        return "organization"
    if any(token in lowered for token in ["project", "app", "assistant", "dash", "deyana"]):
        return "project"
    return "person_or_org"


def unique_entities(candidates: list[EntityCandidate]) -> tuple[EntityCandidate, ...]:
    seen: set[tuple[str, str]] = set()
    unique: list[EntityCandidate] = []
    for candidate in candidates:
        key = (candidate.name.lower(), candidate.entity_type)
        if key in seen:
            continue
        seen.add(key)
        unique.append(candidate)
        if len(unique) >= 24:
            break
    return tuple(unique)


def extract_due_date(sentence: str) -> str | None:
    match = DATE_PATTERN.search(sentence)
    return match.group(0) if match else None


def default_content(title: str, summary: str) -> str:
    return f"# {title}\n\n{summary}"
