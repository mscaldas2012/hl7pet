# Quickstart: HL7-PET Playground Web App

Validates the feature end-to-end against the user stories in spec.md. Run from
the repo root, inside this feature's worktree/branch.

## Prerequisites

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin
cd crates/python && maturin develop && cd ../..   # installs `hl7pet` into the venv (spec 6000)
pip install -r playground/requirements.txt          # Flask (+ pytest)
```

## Run the app

```bash
flask --app playground.app run
```

Open `http://localhost:5000` in a browser.

## Story 1 (P1) — flat PATH with line numbers

1. Paste the contents of `fixtures/messages/multi-obx.hl7` into the message box.
2. Enter PATH `OBX-5` and submit.
3. **Expect**: one result row per `OBX` occurrence, each showing its value and a
   1-based line number matching that `OBX` segment's position in the pasted text.
4. Change the PATH to something that doesn't exist in the message (e.g. `ZZZ-1`)
   and resubmit.
5. **Expect**: an explicit "no results" state, not an empty or broken panel, and
   the previous results are fully replaced (not appended to).

## Story 2 (P2) — hierarchy PATH with a profile

1. With the same message box, replace the message with
   `fixtures/messages/basic-hierarchy.hl7`'s contents.
2. Enter PATH `OBR -> OBX-5` and submit **without** loading a profile.
3. **Expect**: a `profile_required`-style message explaining a hierarchy profile
   is needed — not empty/incorrect results.
4. Upload `fixtures/profiles/basic-two-level.json` as the profile and resubmit
   the same PATH.
5. **Expect**: matched child values are shown, scoped per parent `OBR`
   occurrence, with a visible note that line numbers aren't available for
   hierarchy results (FR-005a) — no line numbers rendered as if they were real.

## Story 3 (P3) — bad input handling

1. Enter an invalid PATH, e.g. `OBX[[1]-5`, against any pasted message.
   **Expect**: a specific parse-error message (`path_error`), not a crash or
   generic failure.
2. Paste text with no `MSH` segment at all (e.g. just `OBX|1|...`) and submit any
   PATH. **Expect**: a `scan_error` message distinct from "no results."
3. Upload a non-JSON file (e.g. rename any `.hl7` fixture to `.json` and upload
   it) as the profile, then submit a hierarchy PATH. **Expect**: a
   `profile_error` message, and confirm a non-hierarchy PATH still works
   normally afterward in the same session (the bad profile doesn't wedge the
   app).

## Non-standard delimiters (Edge Case)

1. Paste a message using non-default delimiters, e.g.
   `fixtures/vectors/scanner/` covers this at the corpus level — construct or
   reuse a small message whose `MSH-1`/`MSH-2` declare non-default characters.
2. Submit a PATH that addresses a field using those delimiters.
   **Expect**: correct value and correct line number, proving the playground
   doesn't hardcode `|`/`^~\&` anywhere in its own code (FR-012) and relies
   entirely on `hl7pet`'s own delimiter-aware scanning.

## Automated checks

```bash
cd playground && pytest
```

Covers the `/api/extract` contract (`contracts/playground-api.md`) — one test
per `status` value, run against the Flask test client with no browser involved.
