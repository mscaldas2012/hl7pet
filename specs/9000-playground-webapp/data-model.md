# Data Model: HL7-PET Playground Web App

This app is stateless (research.md #7) — there is no persisted schema. The
"entities" below are the shapes of data as they flow through one request, from
spec.md's Key Entities section.

## HL7 Message (input)

| Field | Type | Notes |
|---|---|---|
| `message` | string | Raw HL7 v2 text pasted by the user. Required on every extraction request. Not stored past the request. |

**Validation**: none performed by the playground itself beyond the 5 MB request
size cap (research.md #6) — malformed content (missing/truncated MSH, etc.) is
detected by `hl7pet.scan()` internally and surfaces as the `scan_error` status
(research.md #4).

## Hierarchy Profile (input, optional)

| Field | Type | Notes |
|---|---|---|
| `profile_file` | uploaded file (`multipart/form-data`) | Optional. JSON, `segmentDefinition` schema (specs `002`/`008`). |
| `profile` (derived) | dict | Result of `json.loads()` on the uploaded file's contents; passed to `hl7pet.get_value_hierarchy`. |

**Validation**: `json.JSONDecodeError` on parse, or an `Hl7ProfileError` raised
by `hl7pet` once parsed (e.g. malformed `segmentDefinition` shape), both map to
the `profile_error` status (research.md #4, #5). Absence is valid — a
non-hierarchy PATH never requires this field.

## PATH Expression (input)

| Field | Type | Notes |
|---|---|---|
| `path` | string | Required on every extraction request. |

**Validation**: Checked for the `"->"` substring (research.md #3) to decide
hierarchy vs. non-hierarchy dispatch before calling `hl7pet`; syntactic validity
of the PATH itself is entirely `hl7pet`'s own concern (`Hl7PathError` /
`Hl7QueryError` → `path_error` status).

## Result Set (output)

The shape returned to the frontend, always as one JSON object with a
discriminated `status` (research.md #4):

| `status` | Shape of the rest of the response |
|---|---|
| `results` | `{ "hierarchy": false, "results": [{"value": string, "line": int}, ...] }` for a non-hierarchy PATH, or `{ "hierarchy": true, "results": [string, ...] }` for a hierarchy PATH (no `line`, per FR-005a) |
| `no_results` | `{}` (no further data — the empty-state UI needs nothing else) |
| `profile_required` \| `path_error` \| `scan_error` \| `profile_error` | `{ "message": string }` — the human-readable error text (research.md #4) |

`results` is always in message order (FR-006), matching `hl7pet`'s own return
order. Each non-hierarchy entry pairs a value with its 1-based source line number
(`LocatedValue.value`/`LocatedValue.line`, spec `1000`); each hierarchy entry is a
bare value string (FR-005a).
