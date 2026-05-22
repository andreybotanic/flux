#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
INDEX = ROOT / "docs" / "plan_index.json"


def fail(message: str) -> None:
    print(f"plan_index validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    data = json.loads(INDEX.read_text(encoding="utf-8"))

    stages = data.get("stages", [])
    if not stages:
        fail("no stages found")

    ids = [stage["id"] for stage in stages]
    id_set = set(ids)
    if len(ids) != len(id_set):
        fail("duplicate stage id")

    for stage in stages:
        sid = stage["id"]
        stage_file = ROOT / stage["file"]
        if not stage_file.exists():
            fail(f"stage file missing for {sid}: {stage['file']}")

        for dep in stage.get("depends_on", []):
            if dep not in id_set:
                fail(f"{sid} depends on unknown stage {dep}")

        for other in stage.get("parallel_with", []):
            if other not in id_set:
                fail(f"{sid} parallel_with unknown stage {other}")

    # Cycle detection.
    deps = {stage["id"]: list(stage.get("depends_on", [])) for stage in stages}
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node: str) -> None:
        if node in visited:
            return
        if node in visiting:
            fail(f"dependency cycle detected at {node}")
        visiting.add(node)
        for dep in deps[node]:
            visit(dep)
        visiting.remove(node)
        visited.add(node)

    for sid in ids:
        visit(sid)

    fixed = data.get("fixed_requirements", {})
    if fixed.get("id_format") != "namespace:path/to/item":
        fail("fixed id_format is missing or incorrect")

    formats = fixed.get("data_formats", {})
    required_formats = {
        "mod_manifest": "TOML",
        "project_runtime_config": "TOML",
        "content_prototypes": "RON",
        "content_patches": "RON",
        "scenario_definitions": "RON",
        "save_manifest": "JSON",
    }
    for key, expected in required_formats.items():
        if formats.get(key) != expected:
            fail(f"data format {key} must be {expected}, got {formats.get(key)!r}")

    print(f"plan_index OK: {len(stages)} stages, {sum(len(v) for v in deps.values())} dependency edges")


if __name__ == "__main__":
    main()
