#!/usr/bin/env python3
"""Edit/repack Novation Circuit Tracks `.circuittrackspack` files.

This is intentionally narrow: the Rust/Kaitai tool remains the parser/analyzer,
while this Python helper handles pack writing because Python's standard library
has mature ZIP mutation primitives and the Rust Kaitai runtime is read-only.

Supported workflows:
  edit      - mutate drum velocity/probability planes inside one packed project
  replace   - replace or add a project slot with an existing `.ncs` session
  generate  - create a coherent drum-backed project from a template project

Both commands write a new pack. The source pack is never modified in place.
"""

from __future__ import annotations

import argparse
import json
import os
import random
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
DRUM_CHOICE_OFF = 0x0CDB4
DRUM_RHYTHM_OFF = 0x0CDD4
DEFAULT_DRUM_CHOICES_OFF = 0x1A278
SYNTH_TRACK_INFO_OFF = 0x0CD64
TRACK_INFO_STRIDE = 8
VEL_LEVELS = [0, 14, 28, 42, 56, 70, 84, 98, 112, 127]


@dataclass(frozen=True)
class PatternEdit:
    track: int
    pattern: int
    velocities: list[int]
    probability: int | None


@dataclass(frozen=True)
class GeneratedSelection:
    drum_choices: list[int]   # kick, snare, hat, perc/open-hat by generated track
    synth_patches: list[int]  # synth track 1 bass-ish, synth track 2 pad/lead-ish


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


def entries(index: dict, key: str) -> list[tuple[int, str]]:
    return [(i, e.get("name", "")) for i, e in enumerate(index.get(key) or [])]


def categorize_samples(index: dict) -> dict[str, list[tuple[int, str]]]:
    cats: dict[str, list[tuple[int, str]]] = {
        "kick": [],
        "snare": [],
        "hat": [],
        "open_hat": [],
        "perc": [],
    }
    for i, name in entries(index, "samples"):
        low = name.lower()
        if "kick" in low:
            cats["kick"].append((i, name))
        elif "snare" in low or "clap" in low:
            cats["snare"].append((i, name))
        elif "open hat" in low or "open ha" in low:
            cats["open_hat"].append((i, name))
        elif "hat" in low:
            cats["hat"].append((i, name))
        elif any(word in low for word in ("perc", "cym", "ride")):
            cats["perc"].append((i, name))
    return cats


def categorize_patches(index: dict) -> dict[str, list[tuple[int, str]]]:
    cats: dict[str, list[tuple[int, str]]] = {"bass": [], "lead": [], "pad": []}
    for i, name in entries(index, "patches"):
        low = name.lower()
        if not name or low == "initial patch":
            continue
        if any(word in low for word in ("sub", "bass", "reese", "acid", "wobble", "dub")):
            cats["bass"].append((i, name))
        if any(word in low for word in ("pad", "bell", "vibes", "stab", "bleep", "tone", "pulse")):
            cats["pad"].append((i, name))
        if any(word in low for word in ("acid", "stab", "blip", "lead", "tone", "vibes", "gasm")):
            cats["lead"].append((i, name))
    return cats


def choose_named(
    rng: random.Random,
    candidates: list[tuple[int, str]],
    role: str,
    prefer: tuple[str, ...] = (),
) -> tuple[int, str]:
    if not candidates:
        raise invalid(f"pack has no usable {role} entries")
    preferred = [c for c in candidates if any(word in c[1].lower() for word in prefer)]
    return rng.choice(preferred or candidates)


def style_preferences(style: str) -> dict[str, tuple[str, ...]]:
    if style == "ambient":
        return {"sample": ("soft",), "bass": ("sub", "detune"), "pad": ("pad", "bell", "rich", "vibes")}
    if style == "jungle":
        return {"sample": ("real", "nad"), "bass": ("jungle", "funk", "sub", "wobble"), "pad": ("stab", "pad", "bleep")}
    if style == "techno":
        return {"sample": ("tight", "nad"), "bass": ("acid", "dist", "tone", "sub"), "pad": ("stab", "blip", "acid")}
    if style == "house":
        return {"sample": ("real", "soft"), "bass": ("bass", "sub", "funk"), "pad": ("pad", "stab", "bell")}
    return {"sample": (), "bass": (), "pad": ()}


def select_for_generation(index: dict, style: str, seed: int | None) -> tuple[GeneratedSelection, dict[str, str]]:
    rng = random.Random(seed)
    prefs = style_preferences(style)
    samples = categorize_samples(index)
    patches = categorize_patches(index)

    kick = choose_named(rng, samples["kick"], "kick sample", prefs["sample"])
    snare = choose_named(rng, samples["snare"], "snare sample", prefs["sample"])
    hat = choose_named(rng, samples["hat"], "hat sample", prefs["sample"])
    perc_pool = samples["open_hat"] or samples["perc"] or samples["hat"]
    perc = choose_named(rng, perc_pool, "perc/open-hat sample", prefs["sample"])

    bass = choose_named(rng, patches["bass"], "bass synth patch", prefs["bass"])
    pad_pool = patches["pad"] or patches["lead"] or patches["bass"]
    pad = choose_named(rng, pad_pool, "pad/lead synth patch", prefs["pad"])

    selection = GeneratedSelection(
        drum_choices=[kick[0], snare[0], hat[0], perc[0]],
        synth_patches=[bass[0], pad[0]],
    )
    names = {
        "kick": kick[1],
        "snare": snare[1],
        "hat": hat[1],
        "perc": perc[1],
        "synth1": bass[1],
        "synth2": pad[1],
    }
    return selection, names


def clear_drum_pattern(data: bytearray, track: int, pattern: int) -> None:
    base = track * TRACK_STRIDE + pattern * PATTERN_STRIDE
    for step in range(STEPS):
        idx = base + step
        data[VELOCITY_OFF + idx] = 0
        data[PROBABILITY_OFF + idx] = 0
        data[DRUM_CHOICE_OFF + idx] = 255  # 255 = use the track default choice
        data[DRUM_RHYTHM_OFF + idx] = 0


def write_hit(data: bytearray, track: int, pattern: int, step: int, velocity: int, probability: int = 7) -> None:
    idx = track * TRACK_STRIDE + pattern * PATTERN_STRIDE + step
    data[VELOCITY_OFF + idx] = velocity
    data[PROBABILITY_OFF + idx] = probability
    data[DRUM_CHOICE_OFF + idx] = 255  # use default_drum_choices[track]
    data[DRUM_RHYTHM_OFF + idx] = 1


def steps_for(style: str, track: int, pattern: int) -> list[tuple[int, int]]:
    busy = pattern % 4
    if track == 0:  # kick
        base = {
            "ambient": [0, 16],
            "jungle": [0, 10, 16, 22],
            "techno": [0, 8, 16, 24],
            "house": [0, 8, 16, 24],
        }.get(style, [0, 8, 16, 24])
        extras = [6, 14, 30] if busy >= 2 else ([14] if busy == 1 else [])
        return [(s, 118 if s % 8 == 0 else 88) for s in base + extras if s < STEPS]
    if track == 1:  # snare
        base = [8, 24] if style != "jungle" else [8, 22]
        extras = [15, 31] if busy >= 2 else []
        return [(s, 112 if s in base else 70) for s in base + extras if s < STEPS]
    if track == 2:  # hat
        spacing = 4 if style == "ambient" else (1 if busy >= 3 else 2)
        return [(s, 72 if s % 4 else 96) for s in range(0, STEPS, spacing)]
    # perc/open hat
    base = [4, 12, 20, 28] if style in ("house", "techno") else [7, 15, 23, 31]
    extras = [3, 19] if busy >= 2 and style != "ambient" else []
    return [(s, 90 if s in base else 56) for s in base + extras if s < STEPS]


def generate_project(data: bytearray, selection: GeneratedSelection, style: str) -> None:
    require_ncs(data, "template session")
    for track, choice in enumerate(selection.drum_choices):
        data[DEFAULT_DRUM_CHOICES_OFF + track] = choice
    for synth_track, patch in enumerate(selection.synth_patches):
        data[SYNTH_TRACK_INFO_OFF + synth_track * TRACK_INFO_STRIDE] = patch

    for pattern in range(PATTERNS):
        for track in range(TRACKS):
            clear_drum_pattern(data, track, pattern)
            for step, velocity in steps_for(style, track, pattern):
                write_hit(data, track, pattern, step, velocity)


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


def first_present_project(zf: zipfile.ZipFile, index: dict) -> int:
    names = set(zf.namelist())
    for i, project in enumerate(index.get("projects") or []):
        if project.get("url") in names:
            return i
    raise invalid("pack has no project_N.ncs entries to use as a template")


def command_generate(args: argparse.Namespace) -> None:
    src_pack = Path(args.src_pack)
    dst_pack = Path(args.dst_pack)
    name = args.name or f"Generated {args.style}"

    with zipfile.ZipFile(src_pack, "r") as zf:
        index = read_index(zf)
        template_index = args.template if args.template is not None else first_present_project(zf, index)
        template_url, project = read_project(zf, index, template_index)
        target_url = project_url(index, args.project)
        existed = target_url in zf.namelist()

    selection, names = select_for_generation(index, args.style, args.seed)
    generate_project(project, selection, args.style)

    index["projects"][args.project]["name"] = name
    index_json = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    replacements = {
        target_url: bytes(project),
        "index.json": index_json,
    }
    write_repacked(src_pack, dst_pack, replacements)

    action = "replaced" if existed else "added"
    print(
        f"repacked {src_pack} -> {dst_pack}: generated {args.style} project "
        f"{args.project} ({target_url}) from template {template_index} ({template_url}) as {name!r}"
    )
    print(f"  action: {action}")
    print(
        "  drums: "
        f"kick={selection.drum_choices[0]} {names['kick']!r}, "
        f"snare={selection.drum_choices[1]} {names['snare']!r}, "
        f"hat={selection.drum_choices[2]} {names['hat']!r}, "
        f"perc={selection.drum_choices[3]} {names['perc']!r}"
    )
    print(
        "  synths: "
        f"track1={selection.synth_patches[0]} {names['synth1']!r}, "
        f"track2={selection.synth_patches[1]} {names['synth2']!r}"
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

    generate = sub.add_parser("generate", help="generate a coherent project from a template and repack it")
    generate.add_argument("src_pack")
    generate.add_argument("dst_pack")
    generate.add_argument("project", type=lambda s: parse_index(s, 64, "project"))
    generate.add_argument("--template", type=lambda s: parse_index(s, 64, "template"), help="template project index (default: first project present)")
    generate.add_argument("--name", help="project name to write into index.json")
    generate.add_argument("--style", choices=("ambient", "house", "jungle", "techno", "random"), default="jungle")
    generate.add_argument("--seed", type=int, help="deterministic selection seed")
    generate.set_defaults(func=command_generate)

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
