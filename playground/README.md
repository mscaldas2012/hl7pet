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
flask --app playground.app run --port 8080
```

Open `http://localhost:8080` in a browser. **Avoid port 5000**: on macOS it's
also claimed by ControlCenter/AirPlay Receiver (on both IPv4 and IPv6), and
since Flask only binds IPv4, a browser that resolves `localhost` to IPv6
first will silently land on AirPlay Receiver instead of this app — you'll get
a `403` that has nothing to do with `hl7pet` or Flask.

Or use the bounce script, which stops any running instance (tracked by
`playground/.bounce.pid`) and starts a fresh one in the background, logging to
`playground/.bounce.log`:

```bash
playground/bounce.sh              # restart on the default port (8080)
playground/bounce.sh --port 5051  # restart on a different port
playground/bounce.sh --stop-only  # just stop, don't restart
```

## Test

```bash
cd playground && pytest
```

## What it does

- Paste a raw HL7 v2 message and a PATH expression → see every matching value,
  each with its 1-based source line number.
- Optionally upload a hierarchy profile (`segmentDefinition` JSON) to run `->`
  PATHs — results show line numbers too, via spec `011-located-hierarchy-api`'s
  `get_value_hierarchy_located`.
- Nothing submitted is persisted; every request is self-contained.

See `specs/9000-playground-webapp/quickstart.md` for a full manual validation
walkthrough, and `specs/9000-playground-webapp/contracts/playground-api.md` for
the `/api/extract` request/response contract.
