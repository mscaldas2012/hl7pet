# Quickstart: Validating This Spec's Deliverables

This is a documentation/data feature — there's no app to run. "Validation" here means
proving the grammar document and conformance vectors actually satisfy the spec's
Success Criteria (SC-001 through SC-004). Each scenario below is runnable and maps to
one SC.

## Prerequisites

- `gh` CLI, authenticated (used earlier in this session to read
  [mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet) source directly —
  confirmed working)
- `sbt` + a JVM (for SC-004, evaluating vectors against the real Scala library). If
  unavailable in your environment, SC-004 falls back to manual verification by
  someone with local Scala tooling (`research.md` Decision 3).
- `python3` (for the JSON Schema check below; any JSON Schema validator works)

None of these are project dependencies — they're one-time tools for producing and
verifying this spec's deliverables, per this spec's Technical Context.

## Scenario 1 — Grammar has no dangling references (SC-002)

Every nonterminal referenced in `contracts/path-grammar.md`'s grammar block must
either be defined in that same document or explicitly delegated to spec `002`.

```bash
# Extract every nonterminal name used on the right-hand side of a "::=" rule,
# then confirm each one also appears as a left-hand-side definition somewhere
# in the document (or is CHILD_PATH's "-> " hierarchy escape, delegated to spec 002).
grep -oE '^[A-Za-z_]+' specs/001-path-grammar-spec/contracts/path-grammar.md | sort -u
```

**Expected outcome**: every name printed also appears to the left of a `::=` in the
same file. If not, SC-002 fails — fix the grammar document before proceeding.

## Scenario 2 — Quick-Start examples all parse (SC-001)

Every PATH string in `SPEC.md` §8's Quick-Start examples must classify cleanly against
`contracts/path-grammar.md` (no undefined productions, no ambiguous fallthrough).

**Manual check**: for each example PATH in `SPEC.md` §8 (e.g. `MSH-12`, `OBX-5`,
`OBR[1] -> OBX-5`, `OBX[@3.1='94500-6']-5`), walk it through the grammar in
`contracts/path-grammar.md` by hand and confirm it matches `PATH` via a defined
sequence of productions. Record any failure — it means either the grammar is
incomplete or the example needs revisiting.

## Scenario 3 — Conformance vectors are schema-valid

Every file under `vectors/` must validate against
`contracts/conformance-vector.schema.json`.

```bash
python3 -c "
import json, glob
try:
    import jsonschema
except ImportError:
    raise SystemExit('pip install jsonschema, or substitute any other JSON Schema validator')

schema = json.load(open('specs/001-path-grammar-spec/contracts/conformance-vector.schema.json'))
validator = jsonschema.Draft202012Validator(schema)

for f in glob.glob('specs/001-path-grammar-spec/vectors/*.json'):
    for record in json.load(open(f)):
        errors = list(validator.iter_errors(record))
        if errors:
            print(f'{f}: {record.get(\"id\", \"?\")}: {[e.message for e in errors]}')
print('done')
"
```

**Expected outcome**: no errors printed. Every vector's `id` is also unique across all
files (schema doesn't enforce cross-file uniqueness — check separately if vectors are
split across multiple files).

## Scenario 4 — Vectors verified against the real Scala library (SC-004)

For each conformance vector:

1. Clone the Scala repo to a scratch location (never committed here):
   ```bash
   git clone https://github.com/mscaldas2012/hl7-pet /tmp/hl7-pet-verify
   cd /tmp/hl7-pet-verify && sbt console
   ```
2. In the `sbt console` REPL, load the vector's `message_ref` content and call the
   real library with the vector's `path` and `method` (`getValue` or `getFirstValue`),
   passing `flags` if present (e.g. `removeEmpty`).
3. Compare the actual output to the vector's `expected` field.
   - **Match**: mark the vector `Verified` (data-model.md's Conformance Vector
     lifecycle).
   - **Mismatch**: do not silently fix the vector. Open a `[NEEDS CLARIFICATION]`
     per spec FR-010, resolve it, then re-verify.
4. Discard the clone (`rm -rf /tmp/hl7-pet-verify`) — nothing about it is retained.

**Expected outcome**: 100% of vectors reach `Verified` before this spec is considered
complete (SC-004).

## Scenario 5 — Invalid-PATH vectors are actually rejected by the grammar

For each vector with `expected: "INVALID"`, manually walk its `path` through
`contracts/path-grammar.md` and confirm it fails to match `PATH` — and specifically
fails for the reason recorded in `grammar_productions` (e.g. `PID[ABC]-1` should fail
at `SEG_IDX`, not somewhere unrelated).

**Expected outcome**: every invalid vector's stated failure point matches manual
grammar walkthrough. This is what makes FR-012 verifiable rather than asserted.
