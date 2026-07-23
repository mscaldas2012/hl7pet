# HL7-PET Project Instructions

## Spec Kit — Roadmap-Aware Feature Numbering

This project uses Spec Kit for spec-driven development, organized by module via
[ROADMAP.md](ROADMAP.md). Spec Kit itself has no concept of "module" — every
`/speckit-specify` call would otherwise just take the next global sequential
number. Do not let it.

Before running `/speckit-specify` (or calling
`.specify/scripts/bash/create-new-feature.sh` directly):

1. Read `ROADMAP.md` to determine which module the new feature belongs to
   (Rust Core, Parsing & Extraction, Validation, De-identification,
   Transformation, File & Batch Utilities, Language Bindings, or
   Cross-cutting/Infra).
2. Use that module's "Next free" number from the table, and pass it explicitly:
   `--number <N> --short-name <short-name>`. Never let the script
   auto-increment across the whole `specs/` directory — that ignores module
   ranges entirely.
3. After the spec is created, update `ROADMAP.md`: bump the module's "Next
   free" number and add a row to the Status table (spec #, module, short name,
   status).
4. If the feature doesn't cleanly fit one module, pick the module that owns
   the primary change and note the secondary module in the spec's own
   `spec.md`, per the convention documented in `ROADMAP.md`.

If a module's range is unclear or about to run out, stop and ask rather than
guessing a number — the ranges are a manual convention, not something the
tooling enforces or validates.
