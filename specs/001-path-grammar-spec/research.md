# Phase 0 Research: PATH Grammar Specification & Conformance Vectors

All items below were genuinely open (no reasonable default existed in the spec) and
are resolved here so Phase 1 design has no outstanding unknowns.

## Decision 1: EBNF notation to use for the grammar document

**Decision**: Use the same informal EBNF-like dialect `SPEC.md` §3.1 already uses
(`::=`, `|`, `[...]` for optional, quoted literals, `{...}` implied by repetition
notes) rather than adopting a formal standard like ISO/IEC 14977 EBNF or ABNF (RFC
5234).

**Rationale**: `SPEC.md` already expresses the grammar in this style; this spec's job
is to formalize and complete it (fill gaps, add the productions FR-001 lists), not
translate it into an unrelated notation. Switching notations would introduce a
translation step where errors could creep in, with no benefit to the actual
consumers (a human implementer per User Story 1, and the conformance vectors
themselves, which are the real machine-checkable artifact).

**Alternatives considered**:
- ABNF (RFC 5234): more rigorous and tool-supported, but heavier than this small
  grammar needs, and unfamiliar relative to `SPEC.md`'s existing style.
- W3C XML-style EBNF: associated with XML/markup grammars; not a natural fit for a
  string micro-language like PATH.

## Decision 2: Source of truth for extracting the actual grammar

**Decision**: The grammar productions and their exact edge-case behavior are derived
primarily by reading the actual PATH-parsing source in
[mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet), using `SPEC.md` §3.1
as a drafting scaffold — not treating `SPEC.md`'s prose as automatically authoritative.
Any point where the source and `SPEC.md` disagree is raised as a `[NEEDS
CLARIFICATION]` per spec FR-010 during task execution, not silently resolved in favor
of either.

**Rationale**: `SPEC.md` is documentation *about* the library, written by a human, and
could itself already contain the very discrepancies FR-010 anticipates. The source is
the actual contract existing callers depend on. `SPEC.md`'s existing production names
and structure are still valuable as an organizing scaffold, so this isn't a
from-scratch re-derivation.

**Alternatives considered**:
- Trust `SPEC.md` alone: rejected — risks encoding an already-stale description as if
  it were verified, defeating the point of SC-004 (vectors verified against the real
  library).
- Trust the source alone, ignore `SPEC.md`: rejected — discards `SPEC.md`'s already
  human-organized production taxonomy for no reason; re-deriving structure from raw
  source is strictly more work with no accuracy benefit.

## Decision 3: How conformance vectors get verified against the Scala baseline (SC-004)

**Decision**: Clone `mscaldas2012/hl7-pet` to a scratch/temporary location outside this
repo (never committed, never a submodule — consistent with the "no hard dependency"
direction already recorded in `ROADMAP.md`), run `sbt console`, exercise each
conformance vector's PATH string against the loaded library, record the actual output
in the vector's `expected` field, then discard the clone. Nothing about the clone
itself is retained — only the vectors and their now-verified expected results.

**Rationale**: Satisfies SC-004 without violating the repo's explicit "no hard
dependency on the Scala repo" constraint. A temporary local clone plus an interactive
`sbt console` session is the lowest-friction way to exercise real library behavior for
a one-time manual verification pass; no new tooling needs to be built.

**Alternatives considered**:
- A standalone verification harness script committed to the Scala repo: rejected as
  unnecessary setup for a manual, one-time pass; also raises the question of who
  maintains it in a repo this project doesn't own.

**Post-execution notes (T017, 2026-07-23)**: In practice, a plain `sbt console`
session was replaced with a small throwaway Scala file
(`VerifyVectors.scala`, written directly into the scratch clone's `src/main/scala/`,
discarded with the rest of the clone) that reads `vectors/valid.json` and
`messages/*.hl7` directly via Jackson and prints one result line per vector — more
reliable than typing expressions interactively into a REPL for 11 vectors. Getting
`sbt` to actually run required three local-only fixes to the scratch clone (never
pushed back to the Scala repo):

1. `project/build.properties` had an **unresolved git merge conflict** committed to
   it (`<<<<<<< HEAD` markers around the `sbt.version` line). It happened to still
   parse (`sbt.version=1.8.0` was read correctly), but that declared sbt 1.8.0
   crashed with a `ClassfileParser` `NoClassDefFoundError` under this environment's
   JDK/launcher combo. Fixed locally by overwriting the file with a clean
   `sbt.version=1.10.5`.
2. Bumping sbt pulled in a newer `scala-library` transitively than the project's
   declared `scalaVersion := "2.13.10"` (an SIP-51 binary-compatibility check
   failure). Fixed locally by bumping `scalaVersion` to `2.13.15` in `build.sbt`.
3. `sbt`'s own launcher picked up a JDK newer than the one installed for this
   project (self-reported as "Homebrew Java 26.0.1", though `/usr/libexec/java_home
   -V` showed only JDK 17 registered) — new enough that `Thread.stop()`, used by an
   unrelated utility class (`ConsoleProgress.scala`, nothing to do with PATH
   parsing), had been removed from the JDK entirely, breaking compilation of the
   whole project. Fixed by exporting `JAVA_HOME` explicitly to the JDK 17 install
   before invoking `sbt`.

None of these are HL7-PET bugs to fix in the Scala repo — they're artifacts of
verifying a 2023-era `sbt`/Scala project with 2026-era tooling. Anyone repeating
this verification later should expect to hit the same three issues and can apply
the same three local-only fixes.
- Fully manual, by a human with local Scala tooling, outside of any automated
  assistance: viable as a fallback if `sbt`/a JVM isn't available in the executing
  environment — noted as a prerequisite check in `quickstart.md` rather than assumed
  away.

## Decision 4: Conformance vector schema format

**Decision**: Formalize FR-011's field table as an actual JSON Schema (2020-12 draft)
file at `contracts/conformance-vector.schema.json`, in addition to (not instead of)
the human-readable table already in `spec.md`.

**Rationale**: FR-005 explicitly requires the vector format to be machine-consumable
by spec `003`; a Markdown table alone isn't machine-checkable. JSON Schema is itself
language-agnostic, consistent with Constitution Principle IV, and lets both this
spec's own vectors and spec `003`'s later CI wiring validate against one shared
definition instead of two independently-maintained descriptions drifting apart.

**Alternatives considered**:
- Prose-only Markdown table: rejected per FR-005's machine-consumability requirement.
- A language-specific schema (TypeScript interface, Protobuf, Scala case class):
  rejected — adds a language dependency this documentation-only spec has no reason to
  require, and would need re-deriving per binding language anyway.

## Decision 5: Where the formal grammar document lives

**Decision**: The grammar itself is its own file, `contracts/path-grammar.md`, rather
than folded into `data-model.md` or left only inline in `spec.md`.

**Rationale**: The plan template's Phase 1 guidance explicitly calls out "grammars for
parsers" as a canonical `contracts/` artifact type. Spec `006` (Rust PATH parser)
depends on this document specifically as its build contract (User Story 1), so it
should be addressable and versionable on its own rather than embedded in a document
that also covers entities/schema.
