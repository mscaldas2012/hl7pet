# Phase 0 Research: HL7-PET Playground Web App

All Technical Context unknowns are resolved below; none required external
investigation beyond reading this repo's own existing `hl7pet` Python binding
(spec `6000`) and its `crates/python/src/lib.rs` contract.

## 1. Web framework: Flask vs. FastAPI vs. plain WSGI

**Decision**: Flask.

**Rationale**: The entire server surface is one page and one JSON endpoint. Flask
gives routing, a dev server, `request.files` multipart handling, and Jinja2
templating out of the box with a minimal dependency footprint — appropriate for a
"quick detour" devtool. Calls into `hl7pet` are synchronous, in-process PyO3 calls
with no I/O to await, so there's no async benefit to buy.

**Alternatives considered**:
- **FastAPI** — adds Pydantic request/response models and ASGI/async machinery
  that a 3-field form doesn't need; its main draw (auto-generated OpenAPI docs) has
  no real audience for a single-endpoint local tool.
- **Plain `http.server`/WSGI** — would mean hand-rolling multipart file-upload
  parsing and static file serving that Flask already provides correctly.

## 2. Frontend: vanilla JS vs. a framework/build step

**Decision**: A single static HTML page, plain CSS, and one vanilla JS file using
`fetch()`. No React/Vue/htmx, no bundler.

**Rationale**: Three inputs (message, optional profile file, PATH) and one results
panel is well within what a hand-written page can do cleanly. Introducing a build
step would be the first one in this repo and adds tooling for no functional gain,
working against "minimalistic."

**Alternatives considered**:
- **A JS framework (React/Vue)** — rejected: no state complexity that warrants it,
  and it would require a `node_modules`/bundler toolchain this Rust+Python repo
  doesn't otherwise need.
- **htmx** — rejected: a new dependency pattern for one page; plain `fetch()` plus
  a small render function is no more code and adds nothing to learn.

## 3. Detecting hierarchy PATHs and choosing which `hl7pet` entry point to call

**Decision**: Before calling into `hl7pet`, the backend checks the submitted PATH
text for the literal `"->"` substring (the hierarchy operator token, per spec
`001`'s grammar, `contracts/path-grammar.md` line 14: `SEGMENT_EXPR " -> "
CHILD_PATH`).

- If **absent** → the PATH is treated as non-hierarchy: call
  `hl7pet.get_value_located(message, path)`, which yields line-numbered results
  (FR-005) directly, ignoring any profile the user may also have loaded (a flat
  PATH doesn't need one).
- If **present**:
  - and no profile was supplied → return the `profile_required` error state
    (FR-008) immediately, without calling `hl7pet` at all.
  - and a profile was supplied → call `hl7pet.get_value_hierarchy(message, path,
    profile)`, which yields values with no line numbers (FR-005a).

**Rationale**: The Python binding has no `is_hierarchy_path` introspection
function, and `get_value`/`get_value_located` already raise `Hl7PathError` for a
hierarchy PATH with a helpful message ("...use `get_value_hierarchy` with a
profile") — but relying on that message's exact text to distinguish "this is a
hierarchy PATH" from "this PATH has a genuine syntax error" is fragile: both cases
raise the same `Hl7PathError` class, and matching on human-readable message
wording breaks silently if that wording ever changes. Checking for `"->"` up front
is a one-line, deterministic check against the one stable, documented syntax
token, and it stays correct even though spec `006` found the real parser tolerates
extra/missing whitespace around the operator (the literal `"->"` characters are
always present in a valid hierarchy PATH regardless of surrounding spacing).

**Alternatives considered**:
- **Always call `get_value`/`get_value_located`, catch `Hl7PathError`, and inspect
  the message text** — rejected: fragile string-matching on an exception message
  not designed as a machine-readable signal (see Rationale).
- **Add an `is_hierarchy_path` helper to the Python binding** — rejected: this
  would be new public-binding surface belonging to the Language Bindings module
  (`6000`-range) or Parsing & Extraction module, not this tooling spec (`9000`);
  the earlier scope decision (see spec.md Assumptions) deliberately keeps this
  feature self-contained with zero core/binding changes. A one-line substring
  check in the playground's own code needs no such addition.

## 4. Error taxonomy: mapping `hl7pet` outcomes to distinct UI states

**Decision**: The backend's JSON response always carries a `status` field, one of:

| `status` | When | Maps to spec requirement |
|---|---|---|
| `results` | `hl7pet` returned a non-`None` value | FR-004 |
| `no_results` | `hl7pet` returned `None` (no match) | FR-007 |
| `profile_required` | PATH contains `"->"` and no profile was supplied | FR-008 |
| `path_error` | `Hl7PathError` or `Hl7QueryError` raised | FR-009 |
| `scan_error` | `Hl7ScanError` raised | FR-010 |
| `profile_error` | `Hl7ProfileError` raised, or the uploaded file isn't valid JSON | FR-011 |

Each error status carries the underlying exception's own message (already
human-readable per that package's contract) rather than a generic "something went
wrong."

**Rationale**: A discriminated `status` field lets the frontend render each case
distinctly (empty-state illustration for `no_results`, a form-level hint for
`profile_required`, an inline error banner for the three error statuses) without
the frontend needing to parse or guess from HTTP status codes alone, and keeps
"no data" (`no_results`) structurally separate from every error case — carrying
Constitution Principle III's exception-free-absence distinction one layer up into
the app's own contract.

**Alternatives considered**:
- **A single generic `error` status with a message string** — rejected: the
  frontend would have no reliable way to distinguish "no results" (fine, expected)
  from a real error (needs different visual treatment per FR-007 vs. FR-009/010/011).
- **Map each case to a distinct HTTP status code only** (200/404/400/422...) —
  rejected: still needs a body to carry the human-readable message and a machine
  field either way; folding both into one JSON `status` field is simpler than
  requiring the frontend to also branch on the HTTP status.

## 5. Profile input handling

**Decision**: The profile is submitted as an uploaded file (`multipart/form-data`),
matching the user's original request ("choose a file with a profile defined").
The backend reads it and calls `json.loads()` itself; a `json.JSONDecodeError` is
caught and mapped to the same `profile_error` status `Hl7ProfileError` would
produce (research.md #4), so "the file isn't JSON" and "the JSON isn't a valid
profile" are reported identically to the user — both are just "this profile file
is bad."

**Rationale**: Parsing JSON before handing it to `hl7pet.get_value_hierarchy`
(which expects a Python dict, not a raw string) is required regardless; catching
the decode error at that same point costs nothing extra and keeps the profile
error path unified.

## 6. Request size limits

**Decision**: Cap total request size (pasted message + uploaded profile file) at
5 MB via Flask's `MAX_CONTENT_LENGTH`. Exceeding it returns a clear "input too
large" error rather than the request hanging or the page becoming unresponsive
(Edge Cases).

**Rationale**: 5 MB comfortably exceeds any realistic hand-authored or
copy-pasted HL7 message or hierarchy profile used for manual PATH testing (this
repo's own largest fixture messages are a few KB), while still catching the "pasted
something huge by accident" case the Edge Cases section calls out.

**Alternatives considered**:
- **No limit** — rejected: an accidental multi-hundred-MB paste could make the
  browser tab itself unresponsive while rendering the textarea, independent of
  server cost.
- **A much smaller cap (e.g. 100 KB)** — rejected: risks rejecting a legitimate
  large hierarchy profile or a batch of concatenated messages someone pastes in
  while experimenting, with no real benefit over 5 MB for a local single-user tool.

## 7. Statelessness / no persistence

**Decision**: The Flask app keeps no database, session store, or on-disk cache of
any submitted message, profile, or PATH. Every request is fully self-contained —
the frontend resubmits the message, profile, and PATH together on every extraction
request.

**Rationale**: Trivially satisfies FR-013 (no persistence) with no cleanup/expiry
logic to design, and matches the "nothing PHI-shaped is retained" assumption in
spec.md directly — there is simply nowhere for it to be retained.
