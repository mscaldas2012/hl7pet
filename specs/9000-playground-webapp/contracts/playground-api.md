# Contract: Playground Extraction Endpoint

One endpoint, matching the single-page app's single action (submit message +
optional profile + PATH, get results).

## `POST /api/extract`

**Request**: `multipart/form-data`

| Field | Required | Notes |
|---|---|---|
| `message` | yes | Raw HL7 v2 text |
| `path` | yes | PATH expression |
| `profile` | no | Uploaded JSON file (`segmentDefinition` schema) |

Combined request size limited to 5 MB (research.md #6); exceeding it yields
HTTP 413 with a `path_error`-shaped-but-distinct body: `{"status": "too_large",
"message": "..."}` (a sixth status value, purely for this transport-level case —
not one of `hl7pet`'s own outcomes).

**Response**: `200 OK` with `Content-Type: application/json` for every case
*except* the 413-too-large case above. The body always has a `status` field
(research.md #4):

### `status: "results"`, non-hierarchy PATH (no `"->"` in `path`)

```json
{
  "status": "results",
  "hierarchy": false,
  "results": [
    {"value": "Positive", "line": 12},
    {"value": "Negative", "line": 18}
  ]
}
```

### `status: "results"`, hierarchy PATH (profile supplied)

```json
{
  "status": "results",
  "hierarchy": true,
  "results": ["Positive", "Negative"]
}
```

### `status: "no_results"`

```json
{"status": "no_results"}
```

### `status: "profile_required"`

Returned when `path` contains `"->"` and no `profile` file was supplied. `hl7pet`
is never called for this case (research.md #3).

```json
{
  "status": "profile_required",
  "message": "This PATH uses hierarchy navigation ('->') and needs a hierarchy profile. Upload one to continue."
}
```

### `status: "path_error"`

Raised for a syntactically invalid PATH (`Hl7PathError`) or a non-numeric
ordering-filter comparison (`Hl7QueryError`).

```json
{
  "status": "path_error",
  "message": "<hl7pet's own exception message>"
}
```

### `status: "scan_error"`

The pasted message failed to scan (e.g. missing/malformed MSH) — `Hl7ScanError`.

```json
{
  "status": "scan_error",
  "message": "<hl7pet's own exception message>"
}
```

### `status: "profile_error"`

The uploaded profile file wasn't valid JSON, or was valid JSON but not a valid
profile (`Hl7ProfileError`).

```json
{
  "status": "profile_error",
  "message": "<json.JSONDecodeError or hl7pet's own exception message>"
}
```

## Behavioral notes

- Every field in the request is resubmitted on every call — the server holds no
  session state between requests (FR-013).
- `results` is always in message order (FR-006); resubmitting with a new
  `path`/`message`/`profile` produces a completely new response, never a merge
  with a prior one (the frontend replaces its results panel wholesale on each
  response, per FR-006 — there is no accumulation on the server side to reason
  about either).
- This is the only endpoint. `GET /` serves the single static page
  (`templates/index.html`); `GET /static/*` serves `style.css`/`app.js`.
