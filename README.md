# ncstool

Rust tooling for Novation Circuit Tracks `.ncs` session files and
`.circuittrackspack` packs.

The parser is generated from `decompiled_validators/ncs.ksy`; the `.ksy` is the
single source of truth for byte offsets. The generated Rust parser is committed,
so normal builds do **not** require Java or Kaitai Struct Compiler.

## Features

- **Analyze `.ncs` sessions**: prints timing, scale, FX, scenes/chains, synth/MIDI
  pattern summaries, and drum pattern ASCII.
- **Clone + edit drum patterns**: copies a session and applies
  `track:pattern:steps[:probability]` edits before writing.
- **Browse `.circuittrackspack` files**: opens the pack ZIP, reads `index.json`,
  parses each present project, and resolves drum sample / synth patch indices to
  human-readable names.
- **Repack `.circuittrackspack` files**: a focused Python writer can edit one
  packed project or add/replace a project slot while preserving the rest of the
  ZIP.
- **Validator-derived parser**: all known validated regions are typed through
  Kaitai-generated Rust; unknown 36-byte per-pattern gaps are carried raw instead
  of guessed.

## Build

```bash
cd ncs_tool
cargo build --release
```

The binary is `ncs_tool/target/release/ncs-tui`.

## Commands

### Analyze a session

```bash
./target/release/ncs-tui path/to/session.ncs
```

Output includes:

- parsed/carried coverage;
- timing / scale / FX;
- scenes and pattern chains;
- synth and MIDI pattern summaries;
- drum pattern ASCII with probability digits.

### Clone and edit drum patterns

```bash
./target/release/ncs-tui clone source.ncs target.ncs "0:0:X...X...X...X..."
```

Multiple edits can be applied in one clone:

```bash
./target/release/ncs-tui clone source.ncs target.ncs \
  "0:0:X...X...X...X..." \
  "1:0:....X.......X..." \
  "2:0:x.x.x.x.x.x.x.x.:5"
```

The edit path parses the mutated session and runs the typed validation subset
before writing the target file. Invalid edits fail before producing a partial
output file.

### Browse a Circuit Tracks pack

```bash
./target/release/ncs-tui pack "DLR  Sofa Sound.circuittrackspack"
```

Pack support expects the Novation Components ZIP layout:

```text
index.json
projects/project_N.ncs
samples/sample_N.wav
patches/patch_N.syx
```

The command processes only project entries that are actually present in the ZIP
and resolves:

- `default_drum_choices` / drum choice indices -> `samples[]` names;
- `synth_track_info.patch` indices -> `patches[]` names.

### Repack a Circuit Tracks pack

Pack writing is handled by `scripts/pack_repack.py`. It is intentionally narrow:
Rust/Kaitai remains the parser/analyzer, and Python handles ZIP mutation.

Edit drum patterns inside an existing packed project:

```bash
python3 scripts/pack_repack.py edit \
  "DLR  Sofa Sound.circuittrackspack" \
  "DLR  Sofa Sound edited.circuittrackspack" \
  6 \
  "0:0:X...X...X...X...:5"
```

Replace or add a project slot with an existing `.ncs`:

```bash
python3 scripts/pack_repack.py replace \
  "DLR  Sofa Sound.circuittrackspack" \
  "DLR  Sofa Sound expanded.circuittrackspack" \
  7 \
  my_new_project.ncs \
  --name "LLM Jam"
```

Use this with an LLM by having the model emit `track:pattern:steps[:probability]`
edits, applying them to a template session, then repacking the result into an
empty or existing project slot.

## Drum pattern edit format

`track:pattern:steps[:probability]`

- `track`: drum track `0..3`
- `pattern`: pattern `0..7`
- `steps`: up to 32 step characters
- `probability`: optional single digit `0..9`; applied to played steps

### Step characters

| Character | Meaning |
|-----------|---------|
| `X` | strong hit, velocity `127` |
| `x` | weak hit, velocity `32` |
| `.` | rest, velocity `0`, probability forced to `0` |
| `0`..`9` | velocity levels `0, 14, 28, 42, 56, 70, 84, 98, 112, 127` |

Examples:

```text
0:0:X...X...X...X...
1:0:x.x.x.x.x.x.x.x.:7
0:0:9.5.7.3.9.5.7.3.
```

## Repository layout

- `ncs_tool/` — Rust CLI crate (`ncs-tui` binary).
- `ncs_tool/src/kaitai/ncs_session.rs` — generated parser; do not edit by hand.
- `ncs_tool/src/session.rs` — typed owned session model + validation adapter.
- `ncs_tool/src/pack.rs` — `.circuittrackspack` ZIP / manifest support.
- `decompiled_validators/ncs.ksy` — authoritative Kaitai spec.
- `decompiled_validators/FORMAT_MAP.md` — validator-derived format notes.
- `decompiled_validators/offsets.toml` — verified/inferred offset audit trail.
- `decompiled_validators/README.md` — provenance and regeneration notes for the
  Novation validator artifact.
- `scripts/regen_kaitai.sh` — regenerates the Rust parser from `ncs.ksy`.
- `scripts/pack_repack.py` — Python pack writer for editing/replacing packed
  projects.
- `test_data/` — sample sessions used by tests.

## Development

Run tests:

```bash
cd ncs_tool
cargo test
```

Regenerate the parser after changing `decompiled_validators/ncs.ksy`:

```bash
cd ..
KSC=/path/to/kaitai-struct-compiler scripts/regen_kaitai.sh
cd ncs_tool
cargo test
```

Normal users and CI do not need the Kaitai compiler; the generated parser is
checked in.

## Format status

Current parser coverage is about **97.3% parsed/carried** for known 160,780-byte
Circuit Tracks sessions. Remaining intentionally-unmodeled bytes:

- per-pattern 36-byte gaps with no validator reads; carried raw for round-trip
  fidelity;
- header feature-flag bytes not yet decoded into the typed model.

The old exploratory Python drum extractor has been removed. The maintained
parser/analyzer is Rust/Kaitai; Python is used only for the focused pack writer
where ZIP serialization is needed.