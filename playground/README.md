# HL7-PET Playground

A minimalistic local web app for interactively running PATH expressions against
a pasted HL7 v2 message, with optional hierarchy-profile support — spec
[`9000-playground-webapp`](../specs/9000-playground-webapp/spec.md).

## Install

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin
(cd crates/python && maturin develop)   # builds hl7pet-core + installs `hl7pet` into the venv (spec 6000)
pip install -r playground/requirements.txt
```

## Run

```bash
flask --app playground.app run
```

Open `http://localhost:5000` in a browser.

## Test

```bash
cd playground && pytest
```

## What it does

- Paste a raw HL7 v2 message and a PATH expression → see every matching value,
  each with its 1-based source line number.
- Optionally upload a hierarchy profile (`segmentDefinition` JSON) to run `->`
  PATHs. Hierarchy results show values only — no line numbers yet (a documented
  limitation, see spec.md FR-005a).
- Nothing submitted is persisted; every request is self-contained.

See `specs/9000-playground-webapp/quickstart.md` for a full manual validation
walkthrough, and `specs/9000-playground-webapp/contracts/playground-api.md` for
the `/api/extract` request/response contract.
