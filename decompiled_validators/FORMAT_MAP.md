# NCS Format Map — from the Novation project-validator WASM

**Source of truth:** `circuit-tracks-project-validator-e83caa525f3f586024af78cebcb33ad4.wasm`
(Novation Components web validator; the exact code that parses/validates `.ncs`
before upload). The binary and its decompilations are **not committed** here —
see `README.md` for the source URL, MD5, and regeneration steps
(`validator.wasm` → `validator.dcmp` via wabt `wasm-decompile`).

The validator `memcpy`s the **entire file** into a scratch buffer, then runs 31
validators over it in file order. Because it is a flat copy,
**validator read-offset == file offset**. Every number below marked VERIFIED was
read directly from the decompiled validator and cross-checked against a real file
(`test_data/Deep.ncs`, 160,780 bytes).

File size constant in the validator: `0x2740C = 160780` — matches our samples exactly.

---

## Orchestrator → validator map (VERIFIED call order)

`run_validators` (`f_jv`, decompile ~line 27378) calls, in this order:

| # | file section              | decompile fn | status |
|---|---------------------------|--------------|--------|
| 1 | header + feature flags    | f_tu | strings only |
| 2 | timing                    | f_ku | **VERIFIED** offsets+ranges |
| 3 | scenes (16×8)             | f_hu | partial |
| 4 | scene chain               | f_cu | partial |
| 5 | pattern chain             | f_wt | partial |
| 6 | synth patterns — steps    | f_qt | **VERIFIED** geometry |
| 7–10 | synth patterns B–E     | f_kt,f_ft,f_xs,f_ms | pending |
| 11 | synth track info          | f_ds | pending |
| 12 | drum patterns — steps     | f_vr | **VERIFIED** geometry+planes |
| 13–16 | drum patterns B–E     | f_nr,f_dr,f_uq,f_lq | pending |
| 17 | drum mute states          | f_aq | pending |
| 18 | default drum choices      | f_pp | pending |
| 19–23 | midi patterns A–E     | f_ep,f_yo,f_qo,f_ho,f_do | pending |
| 24 | midi track info           | f_yn | pending |
| 25 | scale (root/type)         | f_tn | **VERIFIED** offsets+ranges |
| 26 | fx (delay/reverb)         | f_ln | **VERIFIED** offsets+ranges |
| 27 | midi keyboard octaves     | f_cn | pending |
| 28–30 | report helpers        | f_um,f_nm,f_km | n/a |
| 31 | root orchestrator stub    | f_fm | n/a |

---

## VERIFIED offsets, geometry, ranges

### Timing (`f_ku`)
Reads byte `a[52]` = **0x34**. Check `(tempo-40) & 255 <= 200` → **tempo ∈ [40,240]**.
Matches `main.rs` (swing 0x35, swingSyncRate 0x36, spare1 0x38, spare2 0x3C).

### Scale (`f_tn`)
Reads byte `a[158988]` = **0x26D0C** = scaleRoot, check `<= 11` → **root ∈ [0,11]**.
scaleType follows (0x26D0D). Matches `main.rs`.

### FX (`f_ln`)
Reads byte `a[158990]` = **0x26D0E** = delayPreset, check `<= 15` → **delay ∈ [0,15]**.
reverbPreset at 0x26D0F. Matches `main.rs`.

### Drum patterns — steps (`f_vr`)  ← explains our 4 planes
Index formula (decompile line ~18970):
```
i = base + track*13632 + pattern*1704 + step;   // 0x3540, 0x6A8
velocity_plane = i + 52596;                      // 0xCD74
```
Loops: `step != 32`, `pattern != 8`, `track != 4` → **4×8×32**.
Planes (from field strings, 0x20 apart, matching `main.rs`):
velocity 0xCD74, probabilities 0xCD94, drumChoice 0xCDB4, drumRhythm(mask) 0xCDD4.
Rule confirmed by strings: *"drum rhythm mask of zero when velocity is non-zero"*
and *"non-zero drum rhythm when velocity is zero"* → mask and velocity are coupled.

### Synth patterns — steps (`f_qt`)  ← anchors the big pre-drum block
Index formula (decompile line ~22115):
```
p = base + track*25920 + pattern*3240 + step*28;  // 0x6540, 0xCA8, 0x1C
first field at p[740];                              // synth block base = 0x2E4
```
Loops: `note != 6`, `step != 32`, `pattern != 8`, track `0→1` → **2 tracks × 8 patterns × 32 steps × 6 notes**.
Step stride **0x1C (28 bytes)**; first checked field `p[740] <= 63` → synth step probability ∈ [0,63].
Consistency check: `0x2E4 + 2*0x6540 = 0xCD64`, immediately before the drum base 0xCD74. ✓

---

## FIELD SCHEMA — field *names* VERIFIED (from validator symbols); per-field byte offsets PENDING

These are the exact C++ field names embedded in the validator
(`validator::Session::Data`). They tell us *what* every region contains; the
precise intra-struct byte offsets still need per-validator extraction.

```
header.signature, header.featureFlags.{midiTracks,reserved}
timing.{tempo,swing,swingSyncRate,spare1,spare2}
scenes[16].patternChains[?].{start,end,padding}
sceneChain.{start,end,padding}
patternChains[?].{start,end,padding}

synthPatterns[track][pattern]:
    steps[step].stepInfo.{assignedNoteMask, probability}
    steps[step].notes[note].{noteNumber, gate, delay, velocity}   // 6 notes/step
    playbackRange.{start,end}, syncRate, playbackDirection
    automation[lane].values[]
synthTrackInfo[track].{patch, muteState, sidechainPreset}

drumPatterns[track][pattern]:
    velocity[step], probabilities[step], drumChoice[step], drumRhythm[step]  // 4×32 planes
    playbackRange.{start,end}, syncRate, playbackDirection
    automation[lane].values[]
drumMuteStates[track], defaultDrumChoices[track]

midiPatterns[track][pattern]:  (same shape as synthPatterns)
    steps[step].stepInfo.{assignedNoteMask, probability}
    steps[step].notes[note].{noteNumber, gate, delay, velocity}
    playbackRange.{start,end}, syncRate, playbackDirection
    automation[lane].values[]
midiTrackInfo[track].{patch, muteState, sidechainPreset}

scaleRoot, scaleType, delayPreset, reverbPreset, midiKeyboardOctaves[track]
```

### Key structural revelations
- **The per-pattern 1448-byte "tail"** our tool doesn't decode = `playbackRange`,
  `syncRate`, `playbackDirection`, and **`automation[lane].values[]`**. The
  `pitch/decay/distortion/eq` planes in `main.rs` are almost certainly **automation
  lanes**, not real drum step fields — the validator defines no such drum fields.
- **The ~51 KB pre-drum block** (0x2E4–0xCD64) = `synthPatterns` (2×8×32×6-note steps).
- **The post-drum tail** (from ~0x19CCC) = `midiPatterns` + `midiTrackInfo`, then the
  global scalars (scale 0x26D0C, fx 0x26D0E, octaves). NOTE: scale/fx live *inside*
  this tail; midiPatterns does not own the whole range — sub-boundaries PENDING.

---

## Reproduce

See `README.md` (fetch the `.wasm`, then `wasm-decompile`, or load into
Ghidra 12.0 + the wasm plugin — language auto-detects as `Wasm:LE:32`).

---

## HOW TO CONTINUE — proposed plan

The validator gives us a ground-truth oracle: it read every offset we already
use and confirmed them exactly. The path from ~3% → near-total coverage is now
mechanical, not speculative. Proposed phases (each independently shippable):

### Phase A — extract every remaining offset/range from the decompile (no device needed)
For each PENDING validator (synth B–E, synth_track_info, drum B–E, drum_mute,
default_drum_choices, midi A–E, midi_track_info, octaves, header, scenes,
chains), read its index formula + comparison constants exactly as done for
`f_ku`/`f_vr`/`f_qt`/`f_tn`/`f_ln`. Output: a machine-readable `offsets.toml`
(base, strides, per-field offset, valid range) covering the whole file.
*Deliverable:* `offsets.toml` + fill in every "PENDING" above.

### Phase B — decode synth + MIDI patterns in `ncs_tool` (the ~105 KB of unknowns)
Add `SynthData`/`MidiData` parsers using Phase-A geometry
(synth: 2×8×32 steps, 6 notes/step, step stride 0x1C, base 0x2E4;
midi: same shape, base TBD in tail). Parse `stepInfo{assignedNoteMask,
probability}` + `notes[]{noteNumber,gate,delay,velocity}`. Extend the ASCII
view to synth/midi note rows.
*Deliverable:* analyze output covers synth+midi; coverage metric jumps.

### Phase C — decode the per-pattern tail (playbackRange, syncRate, direction, automation)
Reinterpret the misnamed `pitch/decay/distortion/eq` planes as automation lanes;
add `playbackRange/syncRate/playbackDirection`. This closes the 1448-byte
per-pattern gap across drums *and* synth/midi.
*Deliverable:* per-pattern coverage ~100%; drop the bogus plane names.

### Phase D — turn the validator into our test oracle
Port each validator's range check into `ncs_tool` as a `validate` subcommand,
and add a golden test: our warnings MUST match the validator's verdict on
`Deep.ncs`/`Funk.ncs`. Optionally run the real `validator.wasm` headless
(wasmtime/node) to diff our output against Novation's, byte-for-byte.
*Deliverable:* `ncs-tui validate file.ncs`; round-trip guarantee that edited
files still pass Novation's validator (makes `clone` safe for device upload).

### Phase E (optional) — Ghidra deep-dives where the decompile is murky
`automation[lane].values[]` is a nested/variable structure; if wasm-decompile
output is ambiguous, load `validator.wasm` into **Ghidra 12.0 + the wasm plugin**
(setup below) for its stronger decompiler, or cross-check against the ARM
firmware in `fw-v4486-re.gpr` (12.1.2).

### Priority
A → C → B → D. Phase A unblocks everything; Phase C is the highest coverage-per-
effort (regular, repeats ×64 patterns); D makes `clone` trustworthy for uploads.

---

## Toolchain setup (this machine) — VERIFIED WORKING

- **wabt** (prebuilt, no sudo): `~/battlefield/ctre/wabt-1.0.41/bin/` —
  `wasm-decompile` is the primary tool; fastest path for offset extraction.
- **Portable JDK 21**: `~/battlefield/jdk-21.0.11+10` (Ghidra needs Java 21; no sudo).
- **Ghidra 12.1.2**: `~/battlefield/ghidra_install/ghidra_12.1.2_PUBLIC` — for the
  ARM Cortex-M firmware (`fw-v4486-re.gpr`; reset vector 0x0805c00c confirmed).
- **Ghidra 12.0** + **nneonneo/ghidra-wasm-plugin v2.4.0**:
  `~/battlefield/ghidra120_install/ghidra_12.0_PUBLIC` — separate install because
  the plugin pins `version=12.0`; side-by-side avoids a manifest bypass.
  **Proven:** headless import of `validator.wasm` → loader `WebAssembly`,
  language `Wasm:LE:32:default:default`, analysis + save succeeded.

```bash
export JAVA_HOME=~/battlefield/jdk-21.0.11+10
export PATH="$JAVA_HOME/bin:$PATH"
GH120=~/battlefield/ghidra120_install/ghidra_12.0_PUBLIC
"$GH120/support/analyzeHeadless" <proj_dir> ncsval -import validator.wasm -overwrite
```

> Not used: `bethington/ghidra-mcp` (251-tool MCP server). Heavyweight
> (Java+Maven+Docker) and aimed at driving Ghidra from an agent; unnecessary for
> this format work, where the wabt decompile is the shorter path. Revisit only if
> we want live agent-driven exploration of the ARM firmware.
