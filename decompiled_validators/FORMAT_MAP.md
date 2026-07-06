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
| 1 | header + feature flags    | f_tu | **VERIFIED** signature, size, flags==1, sessionColour<=13 |
| 2 | timing                    | f_ku | **VERIFIED** offsets+ranges |
| 3 | scenes (16×8)             | f_hu | **VERIFIED** start/end<=7, pad==0 (128/128) |
| 4 | scene chain               | f_cu | **VERIFIED** start/end scene 0..15, pad==0 |
| 5 | pattern chain             | f_wt | **VERIFIED** start/end<=7, pad==0 |
| 6 | synth patterns — steps    | f_qt | **VERIFIED** geometry+stepInfo+notes |
| 7–10 | synth pattern tail     | f_kt,f_ft,f_xs,f_ms | **VERIFIED** playbackRange/sync/dir/automation(12 lanes) |
| 11 | synth track info          | f_ds | **VERIFIED** patch<128, mute<=1, sidechain<=7 |
| 12 | drum patterns — steps     | f_vr | **VERIFIED** geometry+4 planes |
| 13–16 | drum pattern tail     | f_nr,f_dr,f_uq,f_lq | **VERIFIED** playbackRange/sync/dir/automation(8 lanes) |
| 17 | drum mute states          | f_aq | **VERIFIED** @0x1A274, 0..1 |
| 18 | default drum choices      | f_pp | **VERIFIED** @0x1A278, 0..64 |
| 19–23 | midi patterns          | f_ep,f_yo,f_qo,f_ho,f_do | **VERIFIED** (same shape as synth, base 0x1A27C) |
| 24 | midi track info           | f_yn | **VERIFIED** patch<=7, mute<=1, sidechain<=7 |
| 25 | scale (root/type)         | f_tn | **VERIFIED** offsets+ranges |
| 26 | fx (delay/reverb)         | f_ln | **VERIFIED** offsets+ranges |
| 27 | midi keyboard octaves     | f_cn | parsed @0x26D10 (allowlist range, not asserted) |
| 28–30 | report helpers        | f_um,f_nm,f_km | n/a |
| 31 | root orchestrator stub    | f_fm | n/a |

**Parsed/carried: ~97.3%.** All validator-covered regions are typed in the
Kaitai spec. Remaining unmapped bytes are the per-pattern 36-byte gaps (no
validator; carried raw) and display-name/header semantics beyond the validated
signature/file-size/feature-flags/sessionColour fields. No legacy hand-rolled
parser remains — the tool parses via the Kaitai-generated code.

---

## VERIFIED offsets, geometry, ranges

### Header (`f_tu`)
`signature` @0x00 accepts `USER` or `DEMO`; `file_size` @0x04 must equal
160780; `feature_flags` @0x08 must equal 1 (`midiTracks` bit set, reserved
bits clear); byte @0x0C validates `<=13` and validator strings name it
`sessionColour`.

### Timing (`f_ku`)
Reads byte `a[52]` = **0x34**. Check `(tempo-40) & 255 <= 200` → **tempo ∈ [40,240]**.
Matches `main.rs` (swing 0x35, swingSyncRate 0x36, spare1 0x38, spare2 0x3C).

### Scale (`f_tn`)
Reads **0x26D0C** = scaleRoot, check `<= 11`; scaleType at **0x26D0D**
checks `<=15`.

### FX (`f_ln`)
Reads **0x26D0E** = delayPreset, check `<= 15`; reverbPreset at **0x26D0F**
checks `<=7`.

### Drum patterns — steps (`f_vr`)  ← explains our 4 planes
Index formula (decompile line ~18970):
```
i = base + track*13632 + pattern*1704 + step;   // 0x3540, 0x6A8
velocity_plane = i + 52596;                      // 0xCD74
```
Loops: `step != 32`, `pattern != 8`, `track != 4` → **4×8×32**.
Planes: velocity 0xCD74, probabilities 0xCD94, drumChoice 0xCDB4,
drumRhythm(mask) 0xCDD4. Ranges/rules from `f_vr`: velocity `0..127`;
probability `0..7` on played hits; drumChoice allowlist `{0..63,255}`;
drumRhythm is non-zero iff velocity is non-zero and uses the shared
`{0..127,255}` allowlist.

### Synth patterns — steps (`f_qt`)  ← anchors the big pre-drum block
Index formula (decompile line ~22115):
```
p = base + track*25920 + pattern*3240 + step*28;  // 0x6540, 0xCA8, 0x1C
first field at p[740];                              // synth block base = 0x2E4
```
Loops: `note != 6`, `step != 32`, `pattern != 8`, track `0→1` → **2 tracks × 8 patterns × 32 steps × 6 notes**.
Step stride **0x1C (28 bytes)**. Per-step 28-byte record (byte offsets from `p+740`):
- **+0 `assignedNoteMask`** (range 0..63) — the note loop bit-tests it: `q[740] >> note & 1`.
  VERIFIED against samples: mask bit-count == present-note count, 512/512.
- **+1 `probability`** (range 0..7).
- +2..3 reserved; **+4..27 = 6 notes × {noteNumber, gate, delay, velocity}** (4 bytes each). Active notes validate noteNumber 1..139, gate 1..224, delay 0..5, velocity 0..127.

> Earlier drafts had +0/+1 swapped (probability/mask). Corrected: `+740` is the
> mask (it's what the note loop shifts), `+741` is probability.

### MIDI patterns — steps (`f_ep`)  ← same shape as synth, relocated
Identical geometry to synth (`track*25920 + pattern*3240 + step*28`), block base
**`0x1A27C` (107132)**; stepInfo `+0` mask (0..63) / `+1` probability (0..7); notes `+4`.
VERIFIED 512/512 on both samples. Block ends `0x26CFC`, right before scale `0x26D0C`. ✓
Consistency: synth `0x2E4 + 2*0x6540 = 0xCD64` (before drums 0xCD74). ✓

---

## FIELD SCHEMA — field names and offsets VERIFIED where listed

These are the exact C++ field names embedded in the validator
(`validator::Session::Data`). Conditional rules and allowlists that Kaitai cannot
express directly are enforced by `Session::validate()`.

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
- **Per-pattern automation** is flat `automation[lane].values[]`: drums have
  8 lanes × 192 bytes, synth/MIDI have 12 lanes × 192 bytes. The allowlist is
  exactly `{0..127,255}`; `255` is the unset/no-automation sentinel. Lane target
  labels remain unknown/ordinal.
- **The old `pitch/decay/distortion/eq` drum plane names are gone.** Those bytes
  overlap verified tail/automation data, not step fields.
- **The ~51 KB pre-drum block** (0x2E4–0xCD64) = `synthPatterns` (2×8×32×6-note steps).
- **The post-drum tail** = MIDI patterns at 0x1A27C, MIDI track info at 0x26CFC,
  then scale/fx/octaves at 0x26D0C..0x26D11.

---

## Reproduce

See `README.md` (fetch the `.wasm`, then `wasm-decompile`, or load into
Ghidra 12.0 + the wasm plugin — language auto-detects as `Wasm:LE:32`).

---

## HOW TO CONTINUE

The regular validator-covered layout is now modeled. Remaining useful work:

1. Reconcile header/display-name semantics around byte 0x0C and the padded name
   at 0x10 against Components/device behavior. The validator names byte 0x0C
   `sessionColour` and constrains it to `0..13`; the pack helper also uses it as
   the displayed-name length, so generated names are capped at 13 bytes.
2. Identify the per-pattern 36-byte gaps (`melodic +900..+935`, drum +132..+167)
   if firmware/UI behavior reveals a semantic owner. The validator does not read
   them, so they are carried raw.
3. Add a dedicated CLI `validate` command if users need validation without the
   full analyze dump. `Session::validate()` already contains the typed subset.

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
