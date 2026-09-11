"""Loads `fixtures/messages/perf/corpus-manifest.json` (spec
6001-python-ffi-benchmark, research.md #2) — reimplements
`crates/core/benches/common/corpus.rs`'s `representative_typical_per_type()`/
`unique_by_size_category()`/hierarchy-eligible selection identically, so
both harnesses benchmark the exact same named corpus messages (spec.md
FR-001/FR-002).
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CorpusMessage:
    message_id: str
    message_type: str
    size_category: str
    content: str
    # Raw profile JSON text, ready for json.loads() -- present only for
    # entries the manifest tags with a profileRef (currently just
    # large_hierarchy_028), mirroring corpus.rs's CorpusMessage.profile_json.
    profile_json: str | None


class Corpus:
    def __init__(self, corpus_id: str, messages: list[CorpusMessage]) -> None:
        self.corpus_id = corpus_id
        self.messages = messages

    def hierarchy_eligible(self) -> list[CorpusMessage]:
        return [m for m in self.messages if m.profile_json is not None]

    def representative_typical_per_type(self) -> list[CorpusMessage]:
        """One message per distinct message_type among sizeCategory ==
        "typical" entries -- the first in manifest order, mirroring
        corpus.rs's own selection exactly (research.md #2)."""
        seen: set[str] = set()
        result = []
        for m in self.messages:
            if m.size_category == "typical" and m.message_type not in seen:
                seen.add(m.message_type)
                result.append(m)
        return result

    def unique_by_size_category(self, category: str) -> CorpusMessage:
        matches = [m for m in self.messages if m.size_category == category]
        if not matches:
            raise ValueError(f"no message with sizeCategory {category!r}")
        if len(matches) > 1:
            raise ValueError(
                f"more than one message with sizeCategory {category!r} -- "
                "ambiguous, per corpus.rs's own .findFirst() precedent"
            )
        return matches[0]


def _fixtures_root() -> Path:
    return Path(__file__).resolve().parents[4] / "fixtures"


def load() -> Corpus:
    perf_dir = _fixtures_root() / "messages" / "perf"
    manifest_path = perf_dir / "corpus-manifest.json"
    manifest = json.loads(manifest_path.read_text())

    messages = []
    for entry in manifest["messages"]:
        content = (perf_dir / entry["filePath"]).read_text()
        profile_ref = entry.get("profileRef")
        profile_json = (perf_dir / profile_ref).read_text() if profile_ref else None
        messages.append(
            CorpusMessage(
                message_id=entry["messageId"],
                message_type=entry["messageType"],
                size_category=entry["sizeCategory"],
                content=content,
                profile_json=profile_json,
            )
        )

    return Corpus(corpus_id=manifest["corpusId"], messages=messages)
