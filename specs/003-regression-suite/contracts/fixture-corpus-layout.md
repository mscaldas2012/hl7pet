# Contract: `fixtures/` Corpus Layout

This document is the stable interface Roadmap specs `005`-`009` (Rust core), and
later the Python/Java bindings (module `6000`-`6999`), build their own test suites
against. Changing this layout after this spec is complete is a breaking change to
that interface and should be treated with the same care as any other cross-spec
contract (see `ROADMAP.md`'s Documented Breaking Changes convention).

## Directory tree

```text
fixtures/
├── messages/           # every Golden Message (*.hl7), flat — no per-spec subfolders
├── profiles/           # every Profile (*.json), flat
├── vectors/
│   ├── path/            # spec 001's vector family
│   │   ├── valid.json
│   │   └── invalid.json
│   ├── hierarchy/        # spec 002's vector family
│   │   ├── basic.json
│   │   └── complex.json
│   └── <new-family>/     # later specs (005-009) add new subdirectories here;
│                         # never modify path/ or hierarchy/ to add unrelated cases
├── schemas/
│   ├── conformance-vector.schema.json            # path family
│   ├── hierarchy-conformance-vector.schema.json  # hierarchy family
│   └── <new-family>.schema.json                  # one schema per new family
└── scripts/
    └── validate_corpus.py
```

## Rules for adding to this corpus (for specs `005` onward)

1. **New messages** go directly under `fixtures/messages/` — do not create a
   per-spec subdirectory. Pick a filename that doesn't collide with an existing one;
   the validator does not currently detect content-duplicate messages under
   different names (see spec `003`'s Edge Cases — this is accepted, not a defect).
2. **New vector families** get their own `fixtures/vectors/<family>/` directory and
   `fixtures/schemas/<family>.schema.json`. Every vector record's `id` MUST be
   unique across the *entire* corpus, not just the new family — run
   `validate_corpus.py` locally before opening a PR to confirm.
3. **`message_ref`/`profile_ref` values are relative to `fixtures/`** (e.g.
   `"messages/foo.hl7"`, not `"../messages/foo.hl7"` or an absolute path) — matching
   the convention already used by the `path` and `hierarchy` families.
4. **Existing families' files (`path/`, `hierarchy/`) are not edited** by later
   specs except to fix a genuine error in already-shipped vector content (rare,
   should reference the correcting spec/PR in the commit message) — new coverage
   goes into new vectors, not by repurposing old ones.
5. Every new vector-family schema SHOULD declare its own coverage-dimension field
   (an array of named rule/production strings, mirroring `grammar_productions` /
   `semantic_rules`) so `validate_corpus.py` can be extended later to report on it
   by name instead of leaving it in the "unrecognized family" bucket indefinitely —
   registering a new dimension in the script is a small, additive change left to
   whichever spec introduces the family, not required by spec `003` itself.

## Non-goals

- This contract does not define *how* `crates/core/tests`, the `pytest` suite, or
  Java tests load and execute these vectors against an actual engine — that's each
  of those later specs' own responsibility. This spec only guarantees the corpus is
  present, internally consistent, and load-bearing as a shared input.
- This contract does not require one shared vector schema across all families —
  `path` and `hierarchy` intentionally keep their own schemas (Roadmap spec
  `001`/`002` deliverables), and new families are free to do the same.
