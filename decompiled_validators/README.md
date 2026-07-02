# Decompiled validators — provenance & regeneration

This directory holds **our own analysis** of Novation's `.ncs` project validator.
It does **not** commit any Novation binary or its decompilation — those are
third-party artifacts of unclear redistribution status. Regenerate them locally.

## What's committed
- `FORMAT_MAP.md` — our reverse-engineered NCS format map (offsets, geometry,
  ranges, field schema), derived by reading the validator. Our work; safe to commit.

## What's NOT committed (fetch/regenerate)
- `validator.wasm` — Novation Components project validator (proprietary).
- `validator.dcmp` / `validator.wat` — decompilations of the above.

### Source
- Binary served from Novation Components (public web asset):
  `https://components.novationmusic.com/vendor/circuit-tracks-project-validator-e83caa525f3f586024af78cebcb33ad4.wasm`
  (MD5 `e83caa525f3f586024af78cebcb33ad4`, ~266 KB).
- Also mirrored in `userx14/CircuitTracksReverseEngineering` @
  `Assets/Novation-Components/` (branch `master`), which is where the firmware
  RE and the idea to read this validator originate. Credit to that project.

### Regenerate
```bash
# 1. fetch the validator (proprietary — do not commit the result)
curl -sL 'https://components.novationmusic.com/vendor/circuit-tracks-project-validator-e83caa525f3f586024af78cebcb33ad4.wasm' \
  -o validator.wasm

# 2. decompile with wabt (prebuilt release, no sudo)
#    https://github.com/WebAssembly/wabt/releases
wasm-decompile validator.wasm -o validator.dcmp   # C-like, best for offsets
wasm2wat        validator.wasm -o validator.wat    # full WAT for xref

# 3. (optional) load into Ghidra 12.0 + nneonneo/ghidra-wasm-plugin
#    language auto-detects as Wasm:LE:32:default:default
```

## License / redistribution
The `.wasm` is Novation's; treat it as proprietary. This directory intentionally
keeps only original analysis so the repo carries no opaque third-party binaries.
