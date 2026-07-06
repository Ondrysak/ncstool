#!/usr/bin/env python3
"""Edit/repack Novation Circuit Tracks `.circuittrackspack` files.

This is intentionally narrow: the Rust/Kaitai tool remains the parser/analyzer,
while this Python helper handles pack writing because Python's standard library
has mature ZIP mutation primitives and the Rust Kaitai runtime is read-only.

Supported workflows:
  edit     - mutate drum velocity/probability planes inside one packed project
  replace  - replace or add a project slot with an existing `.ncs` session

Both commands write a new pack. The source pack is never modified in place.
"""

from __future__ import annotations

import argparse
import json
import os
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

FILE_SIZE = 160_780
TRACKS = 4
PATTERNS = 8
STEPS = 32

# Drum-plane offsets used by the Rust clone path; these point at the first step
# of track 0 / pattern 0. Track/pattern/step indexing is added below.
VELOCITY_OFF = 0x0CD74
PROBABILITY_OFF = 0x0CD94
TRACK_STRIDE = 0x3540
PATTERN_STRIDE = 0x06A8
VEL_LEVELS = [0, 14, 28, 42, 56, 70, 84, 98, 112, 127]


@dataclass(frozen=True)
class PatternEdit:
    track: int
    pattern: int
    velocities: list[int]
    probability: int | None


def invalid(message: str) -> ValueError:
    return ValueError(message)


def parse_index(raw: str, limit: int, label: str) -> int:
    try:
        value = int(raw)
    except ValueError as exc:
        raise invalid(f"invalid {label} {raw!r}: expected 0..{limit - 1}") from exc
    if not 0 <= value < limit:
        raise invalid(f"{label} {value} out of range 0..{limit - 1}")
    return value


def step_char_to_velocity(ch: str) -> int:
    if ch == "X":
        return 127
    if ch == "x":
        return 32
    if ch == ".":
        return 0
    if "0" <= ch <= "9":
        return VEL_LEVELS[ord(ch) - ord("0")]
    raise invalid(f"invalid step char {ch!r}: expected 'X', 'x', '.', or '0'..'9'")


def parse_pattern_edit(spec: str) -> PatternEdit:
    parts = spec.split(":", 3)
    if len(parts) < 3:
        raise invalid(f"pattern spec {spec!r} must be track:pattern:steps[:probability]")

    track = parse_index(parts[0], TRACKS, "track")
    pattern = parse_index(parts[1], PATTERNS, "pattern")
    steps = parts[2]
    if not steps:
        raise invalid(f"pattern spec {spec!r} has no steps")
    if len(steps) > STEPS:
        raise invalid(f"too many steps: {len(steps)} (max {STEPS})")
    velocities = [step_char_to_velocity(ch) for ch in steps]

    probability: int | None = None
    if len(parts) == 4:
        probability = parse_index(parts[3], 10, "probability")

    return PatternEdit(track, pattern, velocities, probability)


def require_ncs(data: bytes | bytearray, label: str) -> None:
    if len(data) != FILE_SIZE:
        raise invalid(f"{label} is {len(data)} bytes; expected {FILE_SIZE} bytes")


def apply_pattern_edit(data: bytearray, edit: PatternEdit) -> None:
    base = edit.track * TRACK_STRIDE + edit.pattern * PATTERN_STRIDE
    prob = 7 if edit.probability is None else edit.probability
    for step, velocity in enumerate(edit.velocities):
        idx = base + step
        velocity_pos = VELOCITY_OFF + idx
        probability_pos = PROBABILITY_OFF + idx
        if velocity_pos >= len(data) or probability_pos >= len(data):
            raise invalid(
                "edit offset out of bounds "
                f"(track {edit.track}, pattern {edit.pattern}, step {step})"
            )
        data[velocity_pos] = velocity
        data[probability_pos] = prob if velocity else 0


def apply_pattern_edits(data: bytearray, specs: Iterable[str]) -> list[PatternEdit]:
    require_ncs(data, "project session")
    edits = [parse_pattern_edit(spec) for spec in specs]
    if not edits:
        raise invalid("at least one pattern edit is required")
    for edit in edits:
        apply_pattern_edit(data, edit)
    return edits


def read_index(zf: zipfile.ZipFile) -> dict:
    try:
        with zf.open("index.json") as fh:
            return json.load(fh)
    except KeyError as exc:
        raise invalid("pack is missing index.json") from exc


def project_url(index: dict, project_index: int) -> str:
    projects = index.get("projects") or []
    if project_index >= len(projects):
        raise invalid(f"project index {project_index} not present in index.json")
    url = projects[project_index].get("url")
    if not url:
        raise invalid(f"project index {project_index} has no URL in index.json")
    return url


def read_project(zf: zipfile.ZipFile, index: dict, project_index: int) -> tuple[str, bytearray]:
    url = project_url(index, project_index)
    try:
        data = bytearray(zf.read(url))
    except KeyError as exc:
        raise invalid(f"pack is missing {url}") from exc
    require_ncs(data, url)
    return url, data


def write_repacked(
    src_pack: Path,
    dst_pack: Path,
    replacements: dict[str, bytes],
) -> None:
    same_path = src_pack == dst_pack or (dst_pack.exists() and src_pack.resolve() == dst_pack.resolve())
    if same_path:
        raise invalid("source and destination pack paths must differ")

    dst_pack.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(prefix=f".{dst_pack.name}.", suffix=".tmp", dir=dst_pack.parent)
    os.close(fd)
    tmp_path = Path(tmp_name)

    written: set[str] = set()
    try:
        with zipfile.ZipFile(src_pack, "r") as src, zipfile.ZipFile(tmp_path, "w") as dst:
            for info in src.infolist():
                name = info.filename
                if name in replacements:
                    dst.writestr(info, replacements[name])
                    written.add(name)
                else:
                    dst.writestr(info, src.read(name))

            for name, payload in replacements.items():
                if name in written:
                    continue
                info = zipfile.ZipInfo(name)
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = 0o644 << 16
                dst.writestr(info, payload)

        os.replace(tmp_path, dst_pack)
    except Exception:
        try:
            tmp_path.unlink()
        except FileNotFoundError:
            pass
        raise


def command_edit(args: argparse.Namespace) -> None:
    src_pack = Path(args.src_pack)
    dst_pack = Path(args.dst_pack)
    with zipfile.ZipFile(src_pack, "r") as zf:
        index = read_index(zf)
        url, project = read_project(zf, index, args.project)

    edits = apply_pattern_edits(project, args.edits)
    write_repacked(src_pack, dst_pack, {url: bytes(project)})
    print(
        f"repacked {src_pack} -> {dst_pack}: edited project {args.project} "
        f"({url}) with {len(edits)} pattern edit(s)"
    )


def command_replace(args: argparse.Namespace) -> None:
    src_pack = Path(args.src_pack)
    dst_pack = Path(args.dst_pack)
    session_path = Path(args.session)
    session = session_path.read_bytes()
    require_ncs(session, str(session_path))

    with zipfile.ZipFile(src_pack, "r") as zf:
        index = read_index(zf)
        url = project_url(index, args.project)
        existed = url in zf.namelist()

    replacements: dict[str, bytes] = {url: session}
    if args.name is not None:
        index["projects"][args.project]["name"] = args.name
        replacements["index.json"] = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode("utf-8")

    write_repacked(src_pack, dst_pack, replacements)
    action = "replaced" if existed else "added"
    suffix = f" as {args.name!r}" if args.name is not None else ""
    print(
        f"repacked {src_pack} -> {dst_pack}: {action} project {args.project} "
        f"({url}) from {session_path}{suffix}"
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    edit = sub.add_parser("edit", help="edit drum patterns inside one packed project")
    edit.add_argument("src_pack")
    edit.add_argument("dst_pack")
    edit.add_argument("project", type=lambda s: parse_index(s, 64, "project"))
    edit.add_argument("edits", nargs="+", help="track:pattern:steps[:probability]")
    edit.set_defaults(func=command_edit)

    replace = sub.add_parser("replace", help="replace or add one project .ncs in a pack")
    replace.add_argument("src_pack")
    replace.add_argument("dst_pack")
    replace.add_argument("project", type=lambda s: parse_index(s, 64, "project"))
    replace.add_argument("session")
    replace.add_argument("--name", help="optional project name to write into index.json")
    replace.set_defaults(func=command_replace)

    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    try:
        args.func(args)
    except (OSError, ValueError, zipfile.BadZipFile) as exc:
        parser.exit(1, f"error: {exc}\n")


if __name__ == "__main__":
    main()
