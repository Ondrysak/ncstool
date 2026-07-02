meta:
  id: ncs_session
  title: Novation Circuit Tracks session (.ncs)
  file-extension: ncs
  endian: le
  # Fixed-size format, 160780 bytes. Layout + valid ranges derived from Novation's
  # project-validator WASM and empirically confirmed against Deep.ncs / Funk.ncs.
  # Tiers: fields with `valid:` ranges below are VERIFIED against the validator.
  # Regions marked "pending" are sized/positioned but not yet field-decoded.

doc: |
  Circuit Tracks session file. The device/web validator copies the whole file
  into RAM and runs 31 validators over it in file order, so validator read-offset
  equals file offset. This spec encodes the sections resolved so far.

seq:
  - id: pre_synth
    size: 0x2e4
    doc: header, feature flags, timing, scenes, scene/pattern chains (partially decoded; see instances)
  - id: synth
    type: melodic_block(2)
    doc: 2 synth tracks x 8 patterns x 32 steps (VERIFIED geometry)
  # After synth comes synth_track_info, then the drum region, midi region, and
  # global scalars. Those are exposed via `instances` at absolute offsets below
  # rather than sequentially, to avoid asserting byte-exact tail boundaries we
  # have not yet verified.

instances:
  # ---- timing (VERIFIED tempo range) ----
  tempo:
    pos: 0x34
    type: u1
    valid:
      min: 40
      max: 240
  swing:
    pos: 0x35
    type: u1
  swing_sync_rate:
    pos: 0x36
    type: u1

  # ---- drums: 4 planes, VERIFIED base/stride ----
  drums:
    pos: 0xcd74
    type: drum_block
    doc: 4 tracks x 8 patterns x 32 steps; velocity plane at 0xCD74

  # ---- global scalars (VERIFIED ranges) ----
  scale_root:
    pos: 0x26d0c
    type: u1
    valid:
      min: 0
      max: 11
  scale_type:
    pos: 0x26d0d
    type: u1
  delay_preset:
    pos: 0x26d0e
    type: u1
    valid:
      min: 0
      max: 15
  reverb_preset:
    pos: 0x26d0f
    type: u1
    valid:
      min: 0
      max: 7

types:
  # ---------- synth / midi patterns ----------
  melodic_block:
    params:
      - id: track_count
        type: u4
    seq:
      - id: tracks
        type: melodic_track
        repeat: expr
        repeat-expr: track_count

  melodic_track:
    seq:
      - id: patterns
        type: melodic_pattern
        repeat: expr
        repeat-expr: 8

  melodic_pattern:
    # pattern stride 3240 = 32 steps * 28 (=896) + 2344-byte tail
    # (playbackRange / syncRate / playbackDirection / automation — pending decode)
    seq:
      - id: steps
        type: melodic_step
        repeat: expr
        repeat-expr: 32
      - id: pattern_tail
        size: 3240 - 32 * 28

  melodic_step:
    # 28 bytes per step, laid out: probability(+0), note_mask(+1), reserved(+2..3),
    # then 6 notes (+4..27). Confirmed byte-exact on Deep.ncs:
    #   record@740 = 0f 07 0000 | 415c004d | 445c0051 | 485d002a ...
    #   -> probability=15, note_mask=0x7, note0={65,92,0,77}, ...
    # (Verified: prob<=63 & mask<=7 for all 512 steps; active-note gate/vel<=127.)
    seq:
      - id: probability
        type: u1
        valid:
          min: 0
          max: 63
      - id: note_mask
        type: u1
        valid:
          min: 0
          max: 7
      - id: reserved
        size: 2
      - id: notes
        type: note
        repeat: expr
        repeat-expr: 6

  note:
    seq:
      - id: note_number
        type: u1
        doc: 0 = empty, else MIDI note 1..139
      - id: gate
        type: u1
      - id: delay
        type: u1
      - id: velocity
        type: u1

  # ---------- drums ----------
  drum_block:
    doc: |
      Structure-of-arrays. Each of 4 tracks (stride 0x3540) holds 8 patterns
      (stride 0x6A8); each pattern has four 32-byte plane arrays at +0x00
      (velocity), +0x20 (probability), +0x40 (drumChoice), +0x60 (drumRhythm).
      Modeled per-pattern below; higher-level track indexing done in code.
    seq:
      - id: tracks
        type: drum_track
        repeat: expr
        repeat-expr: 4

  drum_track:
    seq:
      - id: patterns
        type: drum_pattern
        repeat: expr
        repeat-expr: 8
      - id: track_pad
        size: 0x3540 - 8 * 0x6a8
        doc: remainder of track stride after 8 patterns

  drum_pattern:
    seq:
      - id: velocity
        size: 32
      - id: probability
        size: 32
      - id: drum_choice
        size: 32
      - id: drum_rhythm
        size: 32
      - id: pattern_tail
        size: 0x6a8 - 128
        doc: playbackRange / syncRate / playbackDirection / automation (pending decode)
