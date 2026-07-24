#!/usr/bin/env bash
# Repo-wide check: confirms this harness declares the Scala engine as a Maven
# dependency (spec.md FR-002) rather than vendoring its source or pointing a build
# file at a local filesystem path. Run from anywhere; always resolves paths relative
# to the git repository root. Exits non-zero on any violation (SC-005).
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

fail=0

# 1. No vendored Scala source anywhere in the repository.
vendored=$(git ls-files -- '*.scala' | grep -v '^$' || true)
if [ -n "$vendored" ]; then
  echo "FAIL: found vendored .scala source file(s) in the repository:"
  echo "$vendored"
  fail=1
else
  echo "OK: no .scala source files tracked in the repository"
fi

# 2. No build file references a local filesystem path for the Scala engine dependency
#    (a Maven <systemPath>/system-scope dependency, or an sbt/Ivy "file://" style
#    unmanaged-jar reference), as opposed to a normal versioned coordinate.
suspicious=$(git grep -lE '<scope>\s*system\s*</scope>|systemPath|unmanagedBase.*hl7-pet|file://.*hl7-pet' \
  -- '*.xml' '*.sbt' 2>/dev/null || true)
if [ -n "$suspicious" ]; then
  echo "FAIL: found local-filesystem-path dependency reference(s):"
  echo "$suspicious"
  fail=1
else
  echo "OK: no local-filesystem-path dependency references found"
fi

if [ "$fail" -ne 0 ]; then
  echo "verify-no-vendored-source.sh: FAILED"
  exit 1
fi

echo "verify-no-vendored-source.sh: PASSED"
