# Quickstart: Validating This Spec's Deliverables

This spec consolidates data and adds a validation script — there's no app to run.
"Validation" here means proving the `fixtures/` corpus and its CI check actually
satisfy the spec's Success Criteria (SC-001 through SC-004). Each scenario below is
runnable and maps to one SC.

## Prerequisites

- `python3` (3.11+) with `pip install jsonschema` — the only dependency
  `fixtures/scripts/validate_corpus.py` needs (per `research.md` Decision 1).
- No `sbt`/Scala tooling needed here (unlike spec `001`/`002`) — this spec doesn't
  re-verify vectors against the Scala library, only consolidates already-verified
  ones (spec Assumptions).

## Scenario 1 — Nothing was lost or altered in consolidation (SC-001)

```bash
diff <(python3 -c "import json,sys; print(json.dumps(json.load(open(sys.argv[1])), sort_keys=True))" specs/001-path-grammar-spec/vectors/valid.json) \
     <(python3 -c "import json,sys; print(json.dumps(json.load(open(sys.argv[1])), sort_keys=True))" fixtures/vectors/path/valid.json)
# repeat for invalid.json, and for specs/002-hierarchy-semantics/vectors/{basic,complex}.json
```

**Expected outcome**: no diff output for any of the four file pairs — content is
byte-for-byte identical after JSON-normalizing key order (whitespace/formatting may
differ, values must not).

## Scenario 2 — CI validation catches a deliberately broken vector (SC-003, User Story 2)

```bash
cp fixtures/vectors/path/valid.json /tmp/valid.json.bak
python3 -c "
import json
data = json.load(open('fixtures/vectors/path/valid.json'))
data.append(dict(data[0]))  # duplicate an existing id on purpose
json.dump(data, open('fixtures/vectors/path/valid.json', 'w'), indent=2)
"
python3 fixtures/scripts/validate_corpus.py; echo "exit: $?"
mv /tmp/valid.json.bak fixtures/vectors/path/valid.json  # restore
```

**Expected outcome**: exit code `1`, with a printed line naming the duplicated `id`
and both file locations — never a silent pass. After restoring the backup, a re-run
exits `0`.

## Scenario 3 — Coverage is complete immediately after consolidation (SC-004)

```bash
python3 fixtures/scripts/validate_corpus.py
```

**Expected outcome**: exit code `0`; output shows `path` and `hierarchy` both at
`N/N covered` with zero gaps (both source specs already required 100% coverage
before being marked `Complete` in `ROADMAP.md`).

## Scenario 4 — CI runs automatically and scopes to `fixtures/` changes (FR-005, SC-002)

```bash
git log --oneline -1 -- .github/workflows/fixtures-validation.yml
cat .github/workflows/fixtures-validation.yml
```

**Expected outcome**: the workflow file exists, triggers on `pull_request`/`push`
scoped to `fixtures/**` via a `paths:` filter (`research.md` Decision 5), and its
job runs `python3 fixtures/scripts/validate_corpus.py`. Push a trivial change under
`fixtures/` on a branch and confirm the check appears on the resulting PR and
completes in well under a minute.

## Scenario 5 — An unrecognized vector family is reported, not rejected (FR-007)

```bash
mkdir -p fixtures/vectors/demo
cat > fixtures/schemas/demo.schema.json <<'EOF'
{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["id"],"properties":{"id":{"type":"string"}}}
EOF
echo '[{"id":"demo-001"}]' > fixtures/vectors/demo/sample.json
python3 fixtures/scripts/validate_corpus.py; echo "exit: $?"
rm -rf fixtures/vectors/demo fixtures/schemas/demo.schema.json  # cleanup
```

**Expected outcome**: exit code `0` (assuming no id collision); output lists `demo`
under "unrecognized vector families" with its vector count, and does not fail the
run for lacking a registered coverage dimension.
