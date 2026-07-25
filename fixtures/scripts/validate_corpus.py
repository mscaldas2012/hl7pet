#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

# Known vector families and the schema file (under <corpus_root>/schemas/) each
# validates against. A family directory under vectors/ that isn't listed here is
# "unrecognized" per spec FR-007: still schema/reference/uniqueness-checked if it
# has a matching schema file, but excluded from the path/hierarchy coverage report.
KNOWN_FAMILIES = {
    "path": "conformance-vector.schema.json",
    "hierarchy": "hierarchy-conformance-vector.schema.json",
    "scanner": "scanner-conformance-vector.schema.json",
}

# Coverage dimensions: which field on a vector record names the productions/rules
# it exercises, per family (FR-006). Families not listed here have no registered
# coverage dimension and are reported as "unrecognized" instead (FR-007).
COVERAGE_FIELD = {
    "path": "grammar_productions",
    "hierarchy": "semantic_rules",
}


def default_corpus_root() -> Path:
    return Path(__file__).resolve().parent.parent


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the fixtures/ regression-suite corpus (spec 003-regression-suite)."
    )
    parser.add_argument(
        "--corpus-root",
        type=Path,
        default=default_corpus_root(),
        help="Path to the fixtures/ corpus root (default: fixtures/ relative to this script).",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit the coverage report as JSON on stdout instead of a human-readable summary.",
    )
    return parser.parse_args(argv)


def discover_vector_families(corpus_root: Path) -> dict[str, list[Path]]:
    """Map family name -> list of vector JSON files under vectors/<family>/."""
    vectors_dir = corpus_root / "vectors"
    families: dict[str, list[Path]] = {}
    if not vectors_dir.is_dir():
        return families
    for family_dir in sorted(p for p in vectors_dir.iterdir() if p.is_dir()):
        files = sorted(family_dir.glob("*.json"))
        if files:
            families[family_dir.name] = files
    return families


def load_records(files: list[Path]) -> list[tuple[Path, dict]]:
    """Load every JSON record from a list of vector files, tagged with its source file."""
    records: list[tuple[Path, dict]] = []
    for f in files:
        data = json.loads(f.read_text())
        for record in data:
            records.append((f, record))
    return records


def schema_path_for(corpus_root: Path, family: str) -> Path:
    schema_name = KNOWN_FAMILIES.get(family, f"{family}.schema.json")
    return corpus_root / "schemas" / schema_name


def check_schema_conformance(
    corpus_root: Path, families: dict[str, list[Path]]
) -> list[str]:
    """FR-004.1: every record validates against its family's schema."""
    errors: list[str] = []
    for family, files in families.items():
        schema_file = schema_path_for(corpus_root, family)
        if not schema_file.is_file():
            errors.append(
                f"{family}: no schema found at schemas/{schema_file.name} for vector family '{family}'"
            )
            continue
        schema = json.loads(schema_file.read_text())
        validator = Draft202012Validator(schema)
        for f, record in load_records(files):
            record_id = record.get("id", "?")
            for err in validator.iter_errors(record):
                errors.append(f"{f}: {record_id}: {err.message}")
    return errors


def check_references(corpus_root: Path, families: dict[str, list[Path]]) -> list[str]:
    """FR-004.2: message_ref/profile_ref resolve under messages/ and profiles/."""
    errors: list[str] = []
    for family, files in families.items():
        for f, record in load_records(files):
            record_id = record.get("id", "?")
            message_ref = record.get("message_ref")
            if message_ref is not None and not (corpus_root / message_ref).is_file():
                errors.append(
                    f'{f}: {record_id}: message_ref "{message_ref}" not found'
                )
            profile_ref = record.get("profile_ref")
            if profile_ref is not None and not (corpus_root / profile_ref).is_file():
                errors.append(
                    f'{f}: {record_id}: profile_ref "{profile_ref}" not found'
                )
    return errors


def check_uniqueness(families: dict[str, list[Path]]) -> list[str]:
    """FR-003/FR-004.3: every vector id is unique across the whole corpus."""
    errors: list[str] = []
    seen: dict[str, Path] = {}
    for family, files in families.items():
        for f, record in load_records(files):
            record_id = record.get("id")
            if record_id is None:
                continue
            if record_id in seen:
                errors.append(
                    f'duplicate id "{record_id}": {seen[record_id]} and {f}'
                )
            else:
                seen[record_id] = f
    return errors


def count_vectors(families: dict[str, list[Path]]) -> dict[str, int]:
    return {family: len(load_records(files)) for family, files in families.items()}


def known_dimension_values(corpus_root: Path, family: str, field: str) -> list[str]:
    """Pull the enum of valid production/rule names straight from the family's schema,
    so this script never needs editing when spec 001/002's schemas gain new values."""
    schema_file = schema_path_for(corpus_root, family)
    schema = json.loads(schema_file.read_text())
    prop = schema.get("properties", {}).get(field, {})
    return list(prop.get("items", {}).get("enum", []))


def compute_coverage(corpus_root: Path, families: dict[str, list[Path]]) -> dict:
    """FR-006/FR-007: per-dimension coverage for known families, plus a list of any
    vector family with no registered coverage dimension (reported, never rejected)."""
    dimensions: dict[str, dict] = {}
    unrecognized_families: list[dict] = []

    for family, files in families.items():
        field = COVERAGE_FIELD.get(family)
        if field is None:
            unrecognized_families.append(
                {"family": family, "vector_count": len(load_records(files))}
            )
            continue

        known_values = known_dimension_values(corpus_root, family, field)
        covered: dict[str, list[str]] = {name: [] for name in known_values}
        for f, record in load_records(files):
            record_id = record.get("id", "?")
            for name in record.get(field, []) or []:
                covered.setdefault(name, []).append(record_id)
        gaps = sorted(name for name in known_values if not covered.get(name))
        dimensions[family] = {
            "dimension": field,
            "covered": {k: v for k, v in covered.items() if v},
            "gaps": gaps,
        }

    return {"dimensions": dimensions, "unrecognized_families": unrecognized_families}


def print_human_readable(
    families: dict[str, list[Path]], coverage: dict
) -> None:
    counts = count_vectors(families)
    total = sum(counts.values())
    print("fixtures/ validation")
    print(f"  schema:      {total}/{total} vectors valid")
    print(f"  references:  {total}/{total} message_ref/profile_ref resolved")
    print(f"  ids:         {total}/{total} unique")
    print("  coverage:")
    for family, report in coverage["dimensions"].items():
        known_total = len(report["gaps"]) + len(report["covered"])
        covered_count = len(report["covered"])
        print(f"    {family:<10} {covered_count}/{known_total} {report['dimension']} covered")
        for gap in report["gaps"]:
            print(f"      GAP: {gap}")
    if coverage["unrecognized_families"]:
        for entry in coverage["unrecognized_families"]:
            print(
                f"    (unrecognized vector family: {entry['family']}, "
                f"{entry['vector_count']} vectors)"
            )
    else:
        print("    (no unrecognized vector families)")


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    corpus_root: Path = args.corpus_root

    families = discover_vector_families(corpus_root)
    schema_errors = check_schema_conformance(corpus_root, families)
    reference_errors = check_references(corpus_root, families)
    uniqueness_errors = check_uniqueness(families)
    all_errors = schema_errors + reference_errors + uniqueness_errors

    if all_errors:
        for line in all_errors:
            print(line)
        print("FAILED")
        return 1

    coverage = compute_coverage(corpus_root, families)

    if args.json:
        print(json.dumps(coverage, indent=2))
    else:
        print_human_readable(families, coverage)

    has_gap = any(report["gaps"] for report in coverage["dimensions"].values())
    if has_gap:
        if not args.json:
            print("FAILED (coverage gap)")
        return 2

    if not args.json:
        print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
