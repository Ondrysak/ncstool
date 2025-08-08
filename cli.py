#!/usr/bin/env python3
"""
NCS CLI — helper utilities for experimenting with Novation Circuit session (.ncs) files.

This tool exposes several subcommands for interrogating .ncs files.  A key
subcommand, ``drums extract``, extracts drum pattern data (velocity,
probability, drum choice and rhythm mask) from the binary session.  The
implementation is based on reverse‑engineering of the Circuit Tracks session
validator and assumes a fixed layout for the drum track arrays:

* Four drum tracks (tracks=4).
* Eight patterns per track (patterns=8).
* Thirty‑two steps per pattern (steps=32).
* Each track occupies 0x3540 bytes in the drum arrays (track_stride).
* Each pattern occupies 0x6A8 bytes within a track (pattern_stride).
* Parallel byte arrays store velocity, probability, drum choice and rhythm mask.

For most Circuit Tracks sessions the drum arrays begin at the following
absolute offsets (observed in multiple packs):

* Velocity array: 0x0CD74
* Probability array: 0x0CD94
* Choice array: 0x0CDB4
* Mask array: 0x0CDD4

These defaults are built into the CLI.  You can override them via command
options if your session uses a different layout or if you wish to experiment
with other offsets.  Optionally you can supply a ``--base`` offset to be
added to all array offsets (e.g. when the arrays are relative to a block
offset rather than the start of the file).

Usage examples:

Extract and display drum patterns for all tracks and patterns using the
built‑in defaults:

```
python ncs_cli.py drums extract Funk.ncs
```

Override the mask array offset and write JSON output:

```
python ncs_cli.py drums extract Deep.ncs --mask-off 0x0CDE4 --json deep_drums.json
```

The ASCII output uses Unicode block characters to indicate velocity and
optionally prints probability as a digit appended to the glyph.  Use
``--hide-prob`` to suppress the probability digit.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional

import click

# ---------------------------------------------------------------------------
# Constants and heuristics for the drum layout
#
# These constants describe the observed layout of the drum pattern arrays in
# Circuit Tracks session files (.ncs).  They are derived from reverse
# engineering of the official validator and sample sessions.  The defaults
# cover the vast majority of packs.  When ``--auto-detect`` is enabled the
# CLI will attempt to locate the arrays by scanning the file; if detection
# fails it falls back to these defaults.

# Constants for the default drum layout
DEFAULT_TRACKS = 4
DEFAULT_PATTERNS = 8
DEFAULT_STEPS = 32
DEFAULT_TRACK_STRIDE = 0x3540
DEFAULT_PATTERN_STRIDE = 0x06A8
DEFAULT_VEL_OFF = 0x0CD74
DEFAULT_PROB_OFF = 0x0CD94
DEFAULT_CHOICE_OFF = 0x0CDB4
DEFAULT_MASK_OFF = 0x0CDD4

# The distance between the default mask and choice offsets, etc.  These
# differences are used to derive the other offsets from a detected mask base.
_DEFAULT_OFF_DIFFS = {
    "vel": DEFAULT_VEL_OFF - DEFAULT_MASK_OFF,
    "prob": DEFAULT_PROB_OFF - DEFAULT_MASK_OFF,
    "choice": DEFAULT_CHOICE_OFF - DEFAULT_MASK_OFF,
}


def _detect_drum_offsets(data: bytes, tracks: int, patterns: int, steps: int,
                         mask_off_hint: Optional[int] = None, debug: bool = False) -> Optional[Dict[str, int]]:
    """Attempt to detect the drum array offsets in ``data``.

    The detection heuristic scans the file for a candidate mask array.  The
    mask array should be ``tracks * patterns * steps`` bytes long and be
    comprised mostly of zeros with occasional small non‑zero values.  A mask
    byte of zero indicates no drum hit for that step.

    Parameters
    ----------
    data: bytes
        The entire session file contents.
    tracks: int
        Number of drum tracks (typically 4).
    patterns: int
        Number of patterns per track (typically 8).
    steps: int
        Number of steps per pattern (typically 32).
    mask_off_hint: Optional[int]
        If provided, this offset will be checked first before scanning the
        entire file.  It should point near where a mask array might begin.
    debug: bool
        When True, prints diagnostic information to stderr.

    Returns
    -------
    Optional[Dict[str, int]]
        A dictionary mapping 'velocity', 'probability', 'choice' and 'mask'
        to their detected offsets, or ``None`` if detection fails.
    """
    length = tracks * patterns * steps

    def is_mask_candidate(seg: bytes) -> bool:
        # Mask array is mostly zeros (no hits) with occasional small non‑zero values.
        if not seg:
            return False
        zero_count = seg.count(0)
        ratio = zero_count / len(seg)
        if ratio < 0.6:
            return False
        # The highest value should be relatively small (masks are bitmasks or
        # small counts).  Accept up to 0x1F to allow for edge cases.
        if max(seg) > 0x1F:
            return False
        return True

    # If a hint is provided, check it first
    candidates = []
    if mask_off_hint is not None:
        if mask_off_hint >= 0 and mask_off_hint + length <= len(data):
            seg = data[mask_off_hint:mask_off_hint + length]
            if is_mask_candidate(seg):
                candidates.append(mask_off_hint)

    # Scan the file for mask candidate arrays if no hint succeeded
    if not candidates:
        # Step through the file in increments of 16 bytes to speed scanning
        for off in range(0, len(data) - length, 16):
            seg = data[off:off + length]
            if is_mask_candidate(seg):
                candidates.append(off)
                # Break after finding the first plausible candidate to avoid
                # false positives; additional candidates can be considered
                break

    if debug:
        click.echo(f"[debug] detected mask candidates: {candidates}")

    if not candidates:
        return None

    mask_off = candidates[0]

    # Derive other offsets based on known differences to mask
    vel_off = mask_off + _DEFAULT_OFF_DIFFS["vel"]
    prob_off = mask_off + _DEFAULT_OFF_DIFFS["prob"]
    choice_off = mask_off + _DEFAULT_OFF_DIFFS["choice"]

    # Ensure offsets are within file
    for name, off in {
        "velocity": vel_off,
        "probability": prob_off,
        "choice": choice_off,
        "mask": mask_off,
    }.items():
        if off < 0 or off + length > len(data):
            if debug:
                click.echo(f"[debug] derived {name} offset 0x{off:X} is out of bounds, aborting detection")
            return None

    if debug:
        click.echo(
            f"[debug] auto-detected offsets: velocity=0x{vel_off:X}, probability=0x{prob_off:X}, "
            f"choice=0x{choice_off:X}, mask=0x{mask_off:X}"
        )

    return {
        "velocity": vel_off,
        "probability": prob_off,
        "choice": choice_off,
        "mask": mask_off,
    }


def _parse_int(x: Optional[str]) -> Optional[int]:
    """Parse a string containing a decimal or hexadecimal integer."""
    if x is None:
        return None
    x = x.strip()
    base = 16 if x.lower().startswith("0x") else 10
    return int(x, base)


def _read_u8(buf: bytes, off: int) -> int:
    if off < 0 or off >= len(buf):
        raise IndexError(f"read past end of buffer at 0x{off:X}")
    return buf[off]


def _idx(track: int, pattern: int, step: int, track_stride: int, pattern_stride: int) -> int:
    return track * track_stride + pattern * pattern_stride + step


def _clamp(x: int, lo: int, hi: int) -> int:
    return max(lo, min(hi, x))


def _step_symbol(velocity: int, probability: Optional[int] = None) -> str:
    """Return a simple glyph for a step using velocity and probability.

    Velocity 0 returns a dot.  For velocity > 0 a bar is chosen from
    ▁▃▅█ according to velocity.  If probability is supplied it is appended
    as a digit (0..7).
    """
    if velocity == 0:
        return "."
    levels = "▁▃▅█"
    idx = _clamp((velocity * len(levels)) // 128, 0, len(levels) - 1)
    sym = levels[idx]
    if probability is not None:
        return f"{sym}{probability}"
    return sym


def _render_ascii(pattern_steps: List[Dict[str, int]], show_prob: bool = True) -> str:
    return " ".join(
        _step_symbol(step.get("velocity", 0), step.get("probability") if show_prob else None)
        for step in pattern_steps
    )


def extract_drum_patterns(
    data: bytes,
    *,
    base: int = 0,
    vel_off: int,
    prob_off: Optional[int],
    choice_off: Optional[int],
    mask_off: Optional[int],
    track_stride: int = DEFAULT_TRACK_STRIDE,
    pattern_stride: int = DEFAULT_PATTERN_STRIDE,
    tracks: int = DEFAULT_TRACKS,
    patterns: int = DEFAULT_PATTERNS,
    steps: int = DEFAULT_STEPS,
    track: Optional[int] = None,
    pattern: Optional[int] = None,
) -> Dict:
    """Extract drum pattern information into a nested dictionary.

    Offsets (vel_off, prob_off, etc.) are relative to ``base``.  If a given
    offset is ``None`` that field will not be collected for each step.  Tracks
    and patterns can be restricted by specifying ``track`` and/or ``pattern``.
    """
    if vel_off is None:
        raise click.UsageError("Velocity offset must be provided.")

    def field_offset(field_base: Optional[int], index: int) -> Optional[int]:
        return None if field_base is None else base + field_base + index

    result: Dict[str, List] = {
        "meta": {
            "base": base,
            "track_stride": track_stride,
            "pattern_stride": pattern_stride,
            "tracks": tracks,
            "patterns": patterns,
            "steps": steps,
            "offsets": {
                "velocity": vel_off,
                "probability": prob_off,
                "choice": choice_off,
                "mask": mask_off,
            },
        },
        "tracks": [],
    }
    track_range = range(tracks) if track is None else range(track, track + 1)
    pattern_range = range(patterns) if pattern is None else range(pattern, pattern + 1)

    for t in track_range:
        track_entry = {"track": t, "patterns": []}
        for p in pattern_range:
            steps_list = []
            for s in range(steps):
                offset_idx = _idx(t, p, s, track_stride, pattern_stride)
                step_data: Dict[str, int] = {}
                # velocity (always)
                v_off = field_offset(vel_off, offset_idx)
                step_data["velocity"] = _read_u8(data, v_off) if v_off is not None else 0
                # probability
                if prob_off is not None:
                    p_off = field_offset(prob_off, offset_idx)
                    step_data["probability"] = _read_u8(data, p_off)
                # choice
                if choice_off is not None:
                    c_off = field_offset(choice_off, offset_idx)
                    step_data["choice"] = _read_u8(data, c_off)
                # mask
                if mask_off is not None:
                    m_off = field_offset(mask_off, offset_idx)
                    step_data["mask"] = _read_u8(data, m_off)
                steps_list.append(step_data)
            track_entry["patterns"].append({"pattern": p, "steps": steps_list})
        result["tracks"].append(track_entry)
    return result


@click.group()
@click.version_option(version="0.1.0", prog_name="ncs-cli")
def cli() -> None:
    """NCS toolbox with subcommands for analysis.

    Use ``ncs-cli drums extract`` to extract drum patterns from a session file.
    """


@cli.group()
def drums() -> None:
    """Commands related to drum tracks."""


@drums.command("extract")
@click.argument("ncs_file", type=click.Path(exists=True, dir_okay=False, path_type=Path))
@click.option("--base", "base_hex", default=None,
              help="Base offset added to all field offsets (hex or decimal).")
@click.option("--vel-off", "vel_hex", default=lambda: f"0x{DEFAULT_VEL_OFF:X}", show_default=True,
              help="Velocity array offset (hex or decimal).")
@click.option("--prob-off", "prob_hex", default=lambda: f"0x{DEFAULT_PROB_OFF:X}", show_default=True,
              help="Probability array offset (hex or decimal).")
@click.option("--choice-off", "choice_hex", default=lambda: f"0x{DEFAULT_CHOICE_OFF:X}", show_default=True,
              help="Choice array offset (hex or decimal).")
@click.option("--mask-off", "mask_hex", default=lambda: f"0x{DEFAULT_MASK_OFF:X}", show_default=True,
              help="Mask array offset (hex or decimal).")
@click.option("--track-stride", default=lambda: f"0x{DEFAULT_TRACK_STRIDE:X}", show_default=True,
              help="Stride between tracks in bytes.")
@click.option("--pattern-stride", default=lambda: f"0x{DEFAULT_PATTERN_STRIDE:X}", show_default=True,
              help="Stride between patterns within a track in bytes.")
@click.option("--tracks", default=DEFAULT_TRACKS, show_default=True, help="Number of drum tracks.")
@click.option("--patterns", default=DEFAULT_PATTERNS, show_default=True, help="Number of patterns per track.")
@click.option("--steps", default=DEFAULT_STEPS, show_default=True, help="Steps per pattern.")
@click.option("-t", "track_idx", type=int, help="Restrict extraction to this track (0-based)")
@click.option("-p", "pattern_idx", type=int, help="Restrict extraction to this pattern (0-based)")
@click.option("--json", "json_out", type=click.Path(dir_okay=False, writable=True, path_type=Path),
              help="Write JSON output to file.")
@click.option("--ascii/--no-ascii", default=True, show_default=True,
              help="Print an ASCII representation to stdout.")
@click.option("--show-prob/--hide-prob", default=True, show_default=True,
              help="Include probability digits in ASCII output if probability data is available.")
@click.option("--indent", default=2, show_default=True, help="Indentation for JSON output.")
@click.option("--auto-detect/--no-auto-detect", default=True, show_default=True,
              help="Attempt to locate the drum arrays automatically before using provided offsets.")
@click.option("--debug", is_flag=True, default=False,
              help="Print debug information during auto‑detection.")
def drums_extract(
    ncs_file: Path,
    base_hex: Optional[str],
    vel_hex: str,
    prob_hex: Optional[str],
    choice_hex: Optional[str],
    mask_hex: Optional[str],
    track_stride: str,
    pattern_stride: str,
    tracks: int,
    patterns: int,
    steps: int,
    track_idx: Optional[int],
    pattern_idx: Optional[int],
    json_out: Optional[Path],
    ascii: bool,
    show_prob: bool,
    indent: int,
    auto_detect: bool,
    debug: bool,
) -> None:
    """Extract drum patterns from an .ncs file and output as JSON/ASCII.

    Offsets can be specified as hexadecimal (prefix with ``0x``) or decimal.  If you
    don't provide an offset option it defaults to a commonly observed layout.
    The ``--base`` option can be used to add an offset to all field offsets.
    """
    data = ncs_file.read_bytes()

    base = _parse_int(base_hex) or 0
    ts = _parse_int(track_stride) or 0
    ps = _parse_int(pattern_stride) or 0
    # Parse offsets from command line (may be None)
    vel_off = _parse_int(vel_hex)
    prob_off = _parse_int(prob_hex) if prob_hex else None
    choice_off = _parse_int(choice_hex) if choice_hex else None
    mask_off = _parse_int(mask_hex) if mask_hex else None
    # Auto‑detect offsets if requested
    detected_offsets: Optional[Dict[str, int]] = None
    if auto_detect:
        detected_offsets = _detect_drum_offsets(
            data,
            tracks=tracks,
            patterns=patterns,
            steps=steps,
            mask_off_hint=mask_off,
            debug=debug,
        )
        # If detection succeeded, override offsets
        if detected_offsets:
            vel_off = detected_offsets.get("velocity")
            prob_off = detected_offsets.get("probability")
            choice_off = detected_offsets.get("choice")
            mask_off = detected_offsets.get("mask")
        else:
            # If detection failed but debug requested, notify user
            if debug:
                click.echo("[debug] auto‑detect failed; using provided/default offsets")

    result = extract_drum_patterns(
        data,
        base=base,
        vel_off=vel_off,
        prob_off=prob_off,
        choice_off=choice_off,
        mask_off=mask_off,
        track_stride=ts,
        pattern_stride=ps,
        tracks=tracks,
        patterns=patterns,
        steps=steps,
        track=track_idx,
        pattern=pattern_idx,
    )

    # Write JSON if requested
    if json_out:
        json_out.write_text(json.dumps(result, indent=indent), encoding="utf-8")
        click.echo(f"Wrote JSON to {json_out}")

    # Print ASCII output if requested
    if ascii:
        for trk in result["tracks"]:
            click.echo(f"\n=== DRUM TRACK {trk['track']} ===")
            for patt in trk["patterns"]:
                pnum = patt["pattern"]
                steps_data = patt["steps"]
                click.echo(f"P{pnum:02d}: {_render_ascii(steps_data, show_prob)}")


if __name__ == "__main__":
    cli()