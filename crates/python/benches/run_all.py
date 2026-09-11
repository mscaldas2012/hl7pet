#!/usr/bin/env python3
"""Single entry point for the Python FFI-overhead benchmark harness (spec
6001-python-ffi-benchmark, quickstart.md step 2). Runs parsing/extraction/
hierarchy in sequence against the shared `fixtures/messages/perf/` corpus
and writes one `python-results.json`.

Usage: python crates/python/benches/run_all.py --out <directory>
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extraction  # noqa: E402
import hierarchy  # noqa: E402
import parsing  # noqa: E402
from common import corpus as corpus_module  # noqa: E402
from common.output import write_results  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True, help="run directory to write python-results.json into")
    args = parser.parse_args()

    corpus = corpus_module.load()

    all_results = []
    all_failures = []
    for module in (parsing, extraction, hierarchy):
        results, failures = module.run(corpus)
        all_results.extend(results)
        all_failures.extend(failures)

    write_results(args.out, corpus.corpus_id, all_results, all_failures)

    print(
        f"{len(all_results)} results, {len(all_failures)} engine failure(s)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
