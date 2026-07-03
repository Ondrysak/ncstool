//! Typed NCS session model, derived from Novation's validator WASM.
//!
//! Every offset here is either VERIFIED (read from the decompiled validator and
//! confirmed against the sample files via the validator's range constraints) or
//! marked `// inferred`. See `decompiled_validators/offsets.toml` for the audit
//! trail. Fields whose sub-layout is not yet extracted are left as raw bytes and
//! named `*_raw` so we never fake structure we haven't proven.

use std::io;

pub const FILE_SIZE: usize = 160_780;

fn err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

/// Read a u8 with bounds check.
fn u8_at(d: &[u8], off: usize) -> io::Result<u8> {
    d.get(off).copied().ok_or_else(|| err("offset past end of file"))
}

fn u32le_at(d: &[u8], off: usize) -> io::Result<u32> {
    let b = d.get(off..off + 4).ok_or_else(|| err("u32 past end of file"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

// ---------------- timing ----------------

#[derive(Debug, Clone)]
pub struct Timing {
    pub tempo: u8,           // VERIFIED 40..240
    pub swing: u8,           // inferred 20..80
    pub swing_sync_rate: u8, // inferred 0..7
    pub spare1: u32,
    pub spare2: u32,
}

impl Timing {
    pub fn parse(d: &[u8]) -> io::Result<Self> {
        Ok(Timing {
            tempo: u8_at(d, 0x34)?,
            swing: u8_at(d, 0x35)?,
            swing_sync_rate: u8_at(d, 0x36)?,
            spare1: u32le_at(d, 0x38)?,
            spare2: u32le_at(d, 0x3C)?,
        })
    }
}

// ---------------- synth / midi patterns (VERIFIED geometry) ----------------

pub const SYNTH_BASE: usize = 0x2E4;    // absolute offset of synth track0/pat0/step0 stepInfo
pub const MIDI_BASE: usize = 0x1A27C;   // 107132 — absolute offset of midi stepInfo block
pub const SYNTH_TRACK_STRIDE: usize = 25_920;
pub const PATTERN_STRIDE_SYNTH: usize = 3_240;
pub const STEP_STRIDE: usize = 28;
pub const NOTES_PER_STEP: usize = 6;

/// One note within a synth/midi step. 4 bytes: {noteNumber, gate, delay, velocity}.
#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub note_number: u8, // 0 = empty, else 1..139
    pub gate: u8,
    pub delay: u8,
    pub velocity: u8,
}

impl Note {
    pub fn is_present(&self) -> bool {
        self.note_number != 0
    }
}

/// A synth/midi step: stepInfo (assignedNoteMask + probability) plus up to 6 notes.
/// Byte layout (VERIFIED against the validator + samples):
///   +740 assigned_note_mask (6 bits, range 0..63; bit N => note slot N active)
///   +741 probability (range 0..7)
#[derive(Debug, Clone)]
pub struct MelodicStep {
    pub note_mask: u8,   // VERIFIED 0..63 (bit N => note slot N present); byte +740
    pub probability: u8, // VERIFIED 0..7; byte +741
    pub notes: [Note; NOTES_PER_STEP],
}

impl MelodicStep {
    /// `field_base` is the absolute offset of this step's stepInfo (note_mask byte).
    fn parse(d: &[u8], field_base: usize) -> io::Result<Self> {
        let note_mask = u8_at(d, field_base)?;
        let probability = u8_at(d, field_base + 1)?;
        let mut notes = [Note { note_number: 0, gate: 0, delay: 0, velocity: 0 }; NOTES_PER_STEP];
        for (n, note) in notes.iter_mut().enumerate() {
            let b = field_base + 4 + n * 4;
            *note = Note {
                note_number: u8_at(d, b)?,
                gate: u8_at(d, b + 1)?,
                delay: u8_at(d, b + 2)?,
                velocity: u8_at(d, b + 3)?,
            };
        }
        Ok(MelodicStep { note_mask, probability, notes })
    }

    /// Notes the mask marks active.
    pub fn active_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes
            .iter()
            .enumerate()
            .filter(move |(i, _)| (self.note_mask >> i) & 1 == 1)
            .map(|(_, n)| n)
    }
}

/// Melodic (synth/midi) pattern: 32 steps then a per-pattern tail (VERIFIED).
/// Tail offsets relative to the pattern's stepInfo start (= step 0):
///   +896/+897 playbackRange {start,end} (0..31), +898 syncRate (0..7),
///   +899 playbackDirection (0..3), +900..+935 UNKNOWN (36 bytes),
///   +936.. automation 12 lanes x 192 bytes. Whole record spans 3240 bytes.
#[derive(Debug, Clone)]
pub struct MelodicPattern {
    pub steps: Vec<MelodicStep>, // 32
    pub playback_start: u8,      // +896, 0..31
    pub playback_end: u8,        // +897, 0..31
    pub sync_rate: u8,           // +898, 0..7
    pub playback_direction: u8,  // +899, 0..3
    pub unknown_900_935: [u8; 36], // +900..+935, no validator, carried raw
    pub automation: Vec<[u8; MELODIC_AUTOMATION_LANE_LEN]>, // 12 lanes x 192 @+936
}

pub const MELODIC_AUTOMATION_LANES: usize = 12;
pub const MELODIC_AUTOMATION_LANE_LEN: usize = 192;
const MELODIC_TAIL_PLAYBACK: usize = 896;
const MELODIC_TAIL_AUTOMATION: usize = 936;

#[derive(Debug, Clone)]
pub struct MelodicTrack {
    pub patterns: Vec<MelodicPattern>, // 8
}

/// Parse a melodic block (synth or midi): 2 tracks x 8 patterns x 32 steps + tail.
/// `block_base` is the absolute offset of track0/pattern0/step0's stepInfo.
fn parse_melodic_block(d: &[u8], block_base: usize) -> io::Result<Vec<MelodicTrack>> {
    const TRACKS: usize = 2;
    const PATTERNS: usize = 8;
    const STEPS: usize = 32;
    let mut tracks = Vec::with_capacity(TRACKS);
    for t in 0..TRACKS {
        let mut patterns = Vec::with_capacity(PATTERNS);
        for p in 0..PATTERNS {
            let pat_base = block_base + t * SYNTH_TRACK_STRIDE + p * PATTERN_STRIDE_SYNTH;
            let mut steps = Vec::with_capacity(STEPS);
            for s in 0..STEPS {
                steps.push(MelodicStep::parse(d, pat_base + s * STEP_STRIDE)?);
            }
            let playback_start = u8_at(d, pat_base + MELODIC_TAIL_PLAYBACK)?;
            let playback_end = u8_at(d, pat_base + MELODIC_TAIL_PLAYBACK + 1)?;
            let sync_rate = u8_at(d, pat_base + 898)?;
            let playback_direction = u8_at(d, pat_base + 899)?;
            let mut unknown_900_935 = [0u8; 36];
            for (i, slot) in unknown_900_935.iter_mut().enumerate() {
                *slot = u8_at(d, pat_base + 900 + i)?;
            }
            let mut automation = Vec::with_capacity(MELODIC_AUTOMATION_LANES);
            for lane in 0..MELODIC_AUTOMATION_LANES {
                let mut vals = [0u8; MELODIC_AUTOMATION_LANE_LEN];
                let lane_base = pat_base + MELODIC_TAIL_AUTOMATION + lane * MELODIC_AUTOMATION_LANE_LEN;
                for (v, slot) in vals.iter_mut().enumerate() {
                    *slot = u8_at(d, lane_base + v)?;
                }
                automation.push(vals);
            }
            patterns.push(MelodicPattern {
                steps, playback_start, playback_end, sync_rate, playback_direction,
                unknown_900_935, automation,
            });
        }
        tracks.push(MelodicTrack { patterns });
    }
    Ok(tracks)
}

#[derive(Debug, Clone)]
pub struct SynthData {
    pub tracks: Vec<MelodicTrack>, // 2
}

impl SynthData {
    pub const TRACKS: usize = 2;
    pub const PATTERNS: usize = 8;
    pub const STEPS: usize = 32;
    pub fn parse(d: &[u8]) -> io::Result<Self> {
        Ok(SynthData { tracks: parse_melodic_block(d, SYNTH_BASE)? })
    }
}

#[derive(Debug, Clone)]
pub struct MidiData {
    pub tracks: Vec<MelodicTrack>, // 2
}

impl MidiData {
    pub const TRACKS: usize = 2;
    pub const PATTERNS: usize = 8;
    pub const STEPS: usize = 32;
    pub fn parse(d: &[u8]) -> io::Result<Self> {
        Ok(MidiData { tracks: parse_melodic_block(d, MIDI_BASE)? })
    }
}

// ---------------- drum patterns (VERIFIED geometry, 4 planes) ----------------

pub const DRUM_VELOCITY: usize = 0xCD74;
pub const DRUM_PROBABILITY: usize = 0xCD94; // inferred
pub const DRUM_CHOICE: usize = 0xCDB4; // inferred
pub const DRUM_RHYTHM: usize = 0xCDD4; // inferred
pub const DRUM_TRACK_STRIDE: usize = 0x3540;
pub const PATTERN_STRIDE_DRUM: usize = 0x6A8;

#[derive(Debug, Clone, Copy)]
pub struct DrumStep {
    pub velocity: u8,
    pub probability: u8,
    pub choice: u8,
    pub rhythm: u8,
}

/// Per-pattern tail metadata (VERIFIED offsets, relative to the pattern's
/// velocity-plane base): playbackRange {start,end} @+128/+129 (0..31),
/// syncRate @+130 (0..7), playbackDirection @+131 (0..3). Bytes +132..+167
/// are UNKNOWN (no validator). Automation: 8 lanes x 192 bytes @+168.
#[derive(Debug, Clone)]
pub struct DrumPattern {
    pub steps: Vec<DrumStep>, // 32
    pub playback_start: u8,   // +128, 0..31
    pub playback_end: u8,     // +129, 0..31
    pub sync_rate: u8,        // +130, 0..7
    pub playback_direction: u8, // +131, 0..3
    /// +132..+167 (36 bytes): no validator — carried raw for round-trip fidelity.
    pub unknown_132_167: [u8; 36],
    pub automation: Vec<[u8; 192]>, // 8 lanes x 192 values @+168
}

pub const AUTOMATION_LANES: usize = 8;
pub const AUTOMATION_LANE_LEN: usize = 192;
pub const DRUM_TAIL_PLAYBACK_START: usize = 128;
pub const DRUM_TAIL_AUTOMATION: usize = 168;

#[derive(Debug, Clone)]
pub struct DrumTrack {
    pub patterns: Vec<DrumPattern>, // 8
}

#[derive(Debug, Clone)]
pub struct DrumData {
    pub tracks: Vec<DrumTrack>, // 4
}

impl DrumData {
    pub const TRACKS: usize = 4;
    pub const PATTERNS: usize = 8;
    pub const STEPS: usize = 32;

    pub fn parse(d: &[u8]) -> io::Result<Self> {
        let mut tracks = Vec::with_capacity(Self::TRACKS);
        for t in 0..Self::TRACKS {
            let mut patterns = Vec::with_capacity(Self::PATTERNS);
            for p in 0..Self::PATTERNS {
                let pat_base = t * DRUM_TRACK_STRIDE + p * PATTERN_STRIDE_DRUM;
                let mut steps = Vec::with_capacity(Self::STEPS);
                for s in 0..Self::STEPS {
                    let idx = pat_base + s;
                    steps.push(DrumStep {
                        velocity: u8_at(d, DRUM_VELOCITY + idx)?,
                        probability: u8_at(d, DRUM_PROBABILITY + idx)?,
                        choice: u8_at(d, DRUM_CHOICE + idx)?,
                        rhythm: u8_at(d, DRUM_RHYTHM + idx)?,
                    });
                }
                // tail: offsets relative to the velocity-plane base for this pattern
                let tail = DRUM_VELOCITY + pat_base;
                let playback_start = u8_at(d, tail + DRUM_TAIL_PLAYBACK_START)?;
                let playback_end = u8_at(d, tail + DRUM_TAIL_PLAYBACK_START + 1)?;
                let sync_rate = u8_at(d, tail + 130)?;
                let playback_direction = u8_at(d, tail + 131)?;
                let mut unknown_132_167 = [0u8; 36];
                for (i, slot) in unknown_132_167.iter_mut().enumerate() {
                    *slot = u8_at(d, tail + 132 + i)?;
                }
                let mut automation = Vec::with_capacity(AUTOMATION_LANES);
                for lane in 0..AUTOMATION_LANES {
                    let mut vals = [0u8; AUTOMATION_LANE_LEN];
                    let lane_base = tail + DRUM_TAIL_AUTOMATION + lane * AUTOMATION_LANE_LEN;
                    for (v, slot) in vals.iter_mut().enumerate() {
                        *slot = u8_at(d, lane_base + v)?;
                    }
                    automation.push(vals);
                }
                patterns.push(DrumPattern {
                    steps, playback_start, playback_end, sync_rate, playback_direction,
                    unknown_132_167, automation,
                });
            }
            tracks.push(DrumTrack { patterns });
        }
        Ok(DrumData { tracks })
    }
}

// ---------------- global scalars ----------------

#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub root: u8,       // VERIFIED 0..11
    pub scale_type: u8, // inferred 0..15
}

#[derive(Debug, Clone, Copy)]
pub struct Fx {
    pub delay_preset: u8,  // VERIFIED 0..15
    pub reverb_preset: u8, // inferred 0..7
}

// ---------------- top-level session ----------------

#[derive(Debug, Clone)]
pub struct Session {
    pub timing: Timing,
    pub synth: SynthData,
    pub midi: MidiData,
    pub drums: DrumData,
    pub scale: Scale,
    pub fx: Fx,
    // pending: header, scenes, chains, per-pattern tail + automation, track info, octaves
}
/// Shared range validation for a melodic block (synth or midi), pushing messages
/// prefixed with `kind` into `v`. Ranges are the validator's: probability<=7,
/// note_mask<=63, active-note gate/velocity<=127, note_number 0 or 1..=139.
fn validate_melodic(kind: &str, tracks: &[MelodicTrack], v: &mut Vec<String>) {
    for (ti, track) in tracks.iter().enumerate() {
        for (pi, pat) in track.patterns.iter().enumerate() {
            for (si, step) in pat.steps.iter().enumerate() {
                if step.probability > 7 {
                    v.push(format!("{}[{}][{}].step[{}].probability {} > 7", kind, ti, pi, si, step.probability));
                }
                if step.note_mask > 63 {
                    v.push(format!("{}[{}][{}].step[{}].note_mask {} > 63", kind, ti, pi, si, step.note_mask));
                }
                for (ni, note) in step.notes.iter().enumerate() {
                    if (step.note_mask >> ni) & 1 == 1 {
                        if note.gate > 127 {
                            v.push(format!("{}[{}][{}].step[{}].note[{}].gate {} > 127", kind, ti, pi, si, ni, note.gate));
                        }
                        if note.velocity > 127 {
                            v.push(format!("{}[{}][{}].step[{}].note[{}].velocity {} > 127", kind, ti, pi, si, ni, note.velocity));
                        }
                        if note.note_number != 0 && !(1..=139).contains(&note.note_number) {
                            v.push(format!("{}[{}][{}].step[{}].note[{}].note_number {} not 0 or 1..=139", kind, ti, pi, si, ni, note.note_number));
                        }
                    }
                }
            }
            // per-pattern tail (VERIFIED ranges)
            if pat.playback_start > 31 {
                v.push(format!("{}[{}][{}].playback_start {} > 31", kind, ti, pi, pat.playback_start));
            }
            if pat.playback_end > 31 {
                v.push(format!("{}[{}][{}].playback_end {} > 31", kind, ti, pi, pat.playback_end));
            }
            if pat.sync_rate > 7 {
                v.push(format!("{}[{}][{}].sync_rate {} > 7", kind, ti, pi, pat.sync_rate));
            }
            if pat.playback_direction > 3 {
                v.push(format!("{}[{}][{}].playback_direction {} > 3", kind, ti, pi, pat.playback_direction));
            }
        }
    }
}


impl Session {
    pub fn parse(d: &[u8]) -> io::Result<Self> {
        if d.len() != FILE_SIZE {
            return Err(err("not a Circuit Tracks .ncs (expected 160780 bytes)"));
        }
        Ok(Session {
            timing: Timing::parse(d)?,
            synth: SynthData::parse(d)?,
            midi: MidiData::parse(d)?,
            drums: DrumData::parse(d)?,
            scale: Scale { root: u8_at(d, 0x26D0C)?, scale_type: u8_at(d, 0x26D0D)? },
            fx: Fx { delay_preset: u8_at(d, 0x26D0E)?, reverb_preset: u8_at(d, 0x26D0F)? },
        })
    }

    /// Validate fields against the ranges Novation's validator enforces (verified
    /// subset). Returns the list of violations; empty == passes. Mirrors the WASM
    /// checks so edited files can be gated before writing.
    pub fn validate(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !(40..=240).contains(&self.timing.tempo) {
            v.push(format!("timing.tempo {} out of range 40..=240", self.timing.tempo));
        }
        if self.scale.root > 11 {
            v.push(format!("scale.root {} out of range 0..=11", self.scale.root));
        }
        if self.fx.delay_preset > 15 {
            v.push(format!("fx.delay_preset {} out of range 0..=15", self.fx.delay_preset));
        }
        validate_melodic("synth", &self.synth.tracks, &mut v);
        validate_melodic("midi", &self.midi.tracks, &mut v);
        // Drum planes: velocity is a 7-bit MIDI value (0..=127). This is the plane
        // `clone` edits, so the gate must cover it. (drumChoice/drumRhythm ranges
        // not yet extracted from the validator -> not asserted here.)
        for (ti, track) in self.drums.tracks.iter().enumerate() {
            for (pi, pat) in track.patterns.iter().enumerate() {
                for (si, step) in pat.steps.iter().enumerate() {
                    if step.velocity > 127 {
                        v.push(format!(
                            "drum[{}][{}].step[{}].velocity {} > 127", ti, pi, si, step.velocity));
                    }
                }
                // per-pattern tail (VERIFIED ranges)
                if pat.playback_start > 31 {
                    v.push(format!("drum[{}][{}].playback_start {} > 31", ti, pi, pat.playback_start));
                }
                if pat.playback_end > 31 {
                    v.push(format!("drum[{}][{}].playback_end {} > 31", ti, pi, pat.playback_end));
                }
                if pat.sync_rate > 7 {
                    v.push(format!("drum[{}][{}].sync_rate {} > 7", ti, pi, pat.sync_rate));
                }
                if pat.playback_direction > 3 {
                    v.push(format!("drum[{}][{}].playback_direction {} > 3", ti, pi, pat.playback_direction));
                }
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load a real sample `.ncs` from the sibling `test_data/` dir.
    fn load(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_data")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
    }

    /// Both real files parse, and the reverse-engineered scalar offsets land on
    /// the exact values the validator reports for these two sessions.
    #[test]
    fn session_parses_both_samples() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        let funk = Session::parse(&load("Funk.ncs")).expect("Funk.ncs must parse");

        // Deep.ncs: fully pinned scalars.
        assert_eq!(deep.timing.tempo, 174, "Deep tempo");
        assert_eq!(deep.scale.root, 5, "Deep scale root");
        assert_eq!(deep.scale.scale_type, 15, "Deep scale type");
        assert_eq!(deep.fx.delay_preset, 12, "Deep delay preset");
        assert_eq!(deep.fx.reverb_preset, 3, "Deep reverb preset");

        // Funk.ncs: pinned where the assignment specifies distinct values.
        assert_eq!(funk.timing.tempo, 174, "Funk tempo");
        assert_eq!(funk.scale.root, 0, "Funk scale root");
        assert_eq!(funk.scale.scale_type, 0, "Funk scale type");
        assert_eq!(funk.fx.delay_preset, 9, "Funk delay preset");
        assert_eq!(funk.fx.reverb_preset, 2, "Funk reverb preset");

        // The validator's own range checks must hold on both files.
        for (label, s) in [("Deep", &deep), ("Funk", &funk)] {
            assert!((40..=240).contains(&s.timing.tempo), "{label} tempo out of range");
            assert!(s.scale.root <= 11, "{label} scale root out of range");
            assert!(s.scale.scale_type <= 15, "{label} scale type out of range");
            assert!(s.fx.delay_preset <= 15, "{label} delay preset out of range");
            assert!(s.fx.reverb_preset <= 7, "{label} reverb preset out of range");
        }
    }

    /// Headline invariant: across all 2*8*32 = 512 synth steps in each file,
    /// probability <= 63 and note_mask <= 7 (both are the validator's range
    /// constraints). Also pins the decoded geometry to 2 tracks / 8 patterns /
    /// 32 steps so a stride regression is caught here rather than silently.
    #[test]
    fn synth_stepinfo_ranges_hold_512() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let synth = &session.synth;

            assert_eq!(synth.tracks.len(), SynthData::TRACKS, "{name} synth track count");
            assert_eq!(synth.tracks.len(), 2, "{name} synth track count literal");

            let mut checked = 0usize;
            for (t, track) in synth.tracks.iter().enumerate() {
                assert_eq!(track.patterns.len(), SynthData::PATTERNS, "{name} t{t} pattern count");
                assert_eq!(track.patterns.len(), 8, "{name} t{t} pattern count literal");
                for (p, pat) in track.patterns.iter().enumerate() {
                    assert_eq!(pat.steps.len(), SynthData::STEPS, "{name} t{t} p{p} step count");
                    assert_eq!(pat.steps.len(), 32, "{name} t{t} p{p} step count literal");
                    for (s, step) in pat.steps.iter().enumerate() {
                        assert!(
                            step.probability <= 7,
                            "{name} t{t} p{p} s{s} probability {} > 7",
                            step.probability
                        );
                        assert!(
                            step.note_mask <= 63,
                            "{name} t{t} p{p} s{s} note_mask {} > 63",
                            step.note_mask
                        );
                        checked += 1;
                    }
                }
            }
            assert_eq!(checked, 512, "{name} expected exactly 512 synth steps");
        }
    }

    /// For every mask-active note in Deep.ncs, gate and velocity are valid MIDI
    /// (<= 127) and the note number is either empty (0) or a real pitch (1..=139).
    #[test]
    fn synth_active_notes_in_range() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        for (t, track) in deep.synth.tracks.iter().enumerate() {
            for (p, pat) in track.patterns.iter().enumerate() {
                for (s, step) in pat.steps.iter().enumerate() {
                    for note in step.active_notes() {
                        assert!(
                            note.gate <= 127,
                            "t{t} p{p} s{s} gate {} > 127",
                            note.gate
                        );
                        assert!(
                            note.velocity <= 127,
                            "t{t} p{p} s{s} velocity {} > 127",
                            note.velocity
                        );
                        assert!(
                            note.note_number == 0 || (1..=139).contains(&note.note_number),
                            "t{t} p{p} s{s} note_number {} not 0 or 1..=139",
                            note.note_number
                        );
                    }
                }
            }
        }
    }

    /// Drum plane geometry decodes to 4 tracks / 8 patterns / 32 steps, and the
    /// known strong hit at track2 pattern2 step0 (matching main.rs's drum test)
    /// survives the typed decode.
    #[test]
    fn drum_geometry_and_known_hit() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        let drums = &deep.drums;

        assert_eq!(drums.tracks.len(), DrumData::TRACKS, "drum track count");
        assert_eq!(drums.tracks.len(), 4, "drum track count literal");
        for (t, track) in drums.tracks.iter().enumerate() {
            assert_eq!(track.patterns.len(), DrumData::PATTERNS, "t{t} drum pattern count");
            assert_eq!(track.patterns.len(), 8, "t{t} drum pattern count literal");
            for (p, pat) in track.patterns.iter().enumerate() {
                assert_eq!(pat.steps.len(), DrumData::STEPS, "t{t} p{p} drum step count");
                assert_eq!(pat.steps.len(), 32, "t{t} p{p} drum step count literal");
            }
        }

        assert!(
            drums.tracks[2].patterns[2].steps[0].velocity >= 96,
            "Deep drum track2 pattern2 step0 expected a strong hit (>=96), got {}",
            drums.tracks[2].patterns[2].steps[0].velocity
        );
    }

    /// Session::parse only accepts an exact-size buffer; short/empty input errors
    /// instead of reading out of bounds.
    #[test]
    fn parse_rejects_wrong_size() {
        assert!(Session::parse(&vec![0u8; 100]).is_err(), "100-byte buffer must be rejected");
        assert!(Session::parse(&[]).is_err(), "empty buffer must be rejected");
    }

    /// A populated synth step (note_mask != 0) carries at least one present note,
    /// exercising both `MelodicStep::active_notes` and `Note::is_present`. Deep.ncs
    /// is known to contain such steps (e.g. synth track0).
    #[test]
    fn note_is_present_helper() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");

        // Is there any populated step at all? (Guaranteed for these samples; the
        // early return only guards a hypothetical empty session.)
        let has_populated = deep
            .synth
            .tracks
            .iter()
            .flat_map(|t| &t.patterns)
            .flat_map(|p| &p.steps)
            .any(|s| s.note_mask != 0);
        if !has_populated {
            return;
        }

        // Every populated step's mask must resolve to real notes: at least one
        // masked-active note in the file reports is_present().
        let present = deep
            .synth
            .tracks
            .iter()
            .flat_map(|t| &t.patterns)
            .flat_map(|p| &p.steps)
            .filter(|s| s.note_mask != 0)
            .any(|s| s.active_notes().any(|n| n.is_present()));
        assert!(
            present,
            "populated synth steps existed but none exposed a present active note"
        );
    }

    /// Headline: both real, valid sample files parse and report ZERO
    /// range-check violations. This is the contract `clone` relies on -- an
    /// untouched real session must pass the gate clean. If `validate()` grows a
    /// spurious check (or a real range is decoded wrong), one of these files
    /// reddens here.
    #[test]
    fn real_sessions_pass_validation() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let violations = session.validate();
            assert!(
                violations.is_empty(),
                "{name} is a real valid session but validate() reported {} violation(s): {:?}",
                violations.len(),
                violations
            );
        }
    }

    /// A tempo below the 40..=240 window and one above it are each reported, and
    /// the message names the offending field. Guards the lower and upper edge of
    /// the timing range check.
    #[test]
    fn bad_tempo_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");

        sess.timing.tempo = 10;
        let low = sess.validate();
        assert!(
            low.iter().any(|m| m.contains("tempo")),
            "tempo=10 (below 40) must be reported with a message naming tempo, got {:?}",
            low
        );

        sess.timing.tempo = 241;
        let high = sess.validate();
        assert!(
            high.iter().any(|m| m.contains("tempo")),
            "tempo=241 (above 240) must be reported with a message naming tempo, got {:?}",
            high
        );
    }

    /// A synth step probability past 7 is flagged, and the message names both
    /// the field and the limit it violated.
    #[test]
    fn bad_synth_probability_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.synth.tracks[0].patterns[0].steps[0].probability = 200;

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("probability") && m.contains("7")),
            "probability=200 must be reported naming 'probability' and the '7' limit, got {:?}",
            v
        );
    }

    /// A synth step note_mask past 63 (6 note-slot bits) is flagged with a
    /// message naming the field.
    #[test]
    fn bad_note_mask_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.synth.tracks[0].patterns[0].steps[0].note_mask = 64;

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("note_mask")),
            "note_mask=64 (> 63) must be reported naming note_mask, got {:?}",
            v
        );
    }

    /// Velocity is only checked on mask-active notes. Activate note slot 0
    /// (note_mask bit 0) and push that note's velocity past 127; the violation
    /// must surface. The mask bit chosen matches the note index mutated -- an
    /// inactive note's out-of-range velocity is intentionally ignored, so the
    /// mask/index pairing is load-bearing for this test to have teeth.
    #[test]
    fn bad_active_note_velocity_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        let step = &mut sess.synth.tracks[0].patterns[0].steps[0];
        step.note_mask = 1; // bit 0 active -> notes[0] is checked
        step.notes[0] = Note { note_number: 60, gate: 100, delay: 0, velocity: 200 };

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("velocity")),
            "an active note with velocity=200 must be reported naming velocity, got {:?}",
            v
        );
    }

    /// A drum step velocity past 127 is flagged, and the message identifies it as
    /// a drum-plane velocity (the plane `clone` edits).
    #[test]
    fn bad_drum_velocity_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.drums.tracks[0].patterns[0].steps[0].velocity = 200;

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("drum") && m.contains("velocity")),
            "drum velocity=200 must be reported naming 'drum' and 'velocity', got {:?}",
            v
        );
    }

    /// The two global scalar bounds -- scale.root <= 11 and fx.delay_preset <= 15
    /// -- are each reported with a message naming the specific field.
    #[test]
    fn scale_and_fx_bounds_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");

        sess.scale.root = 12;
        let root = sess.validate();
        assert!(
            root.iter().any(|m| m.contains("scale.root")),
            "scale.root=12 (> 11) must be reported naming scale.root, got {:?}",
            root
        );

        sess.scale.root = 5; // back in range so only delay_preset is out
        sess.fx.delay_preset = 16;
        let delay = sess.validate();
        assert!(
            delay.iter().any(|m| m.contains("delay_preset")),
            "fx.delay_preset=16 (> 15) must be reported naming delay_preset, got {:?}",
            delay
        );
    }

    /// MIDI block geometry decodes to the same 2 tracks / 8 patterns / 32 steps
    /// (512 steps) as the synth block, and across all 512 steps in BOTH real
    /// files every step's probability <= 7 and note_mask <= 63. A wrong MIDI_BASE
    /// or stride would either miscount the geometry or read garbage that blows the
    /// ranges; a re-swap of the two stepInfo bytes would push probability past 7.
    #[test]
    fn midi_geometry_512_and_ranges() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let midi = &session.midi;

            assert_eq!(midi.tracks.len(), 2, "{name} midi track count");

            let mut checked = 0usize;
            for (t, track) in midi.tracks.iter().enumerate() {
                assert_eq!(track.patterns.len(), 8, "{name} midi t{t} pattern count");
                for (p, pat) in track.patterns.iter().enumerate() {
                    assert_eq!(pat.steps.len(), 32, "{name} midi t{t} p{p} step count");
                    for (s, step) in pat.steps.iter().enumerate() {
                        assert!(
                            step.probability <= 7,
                            "{name} midi t{t} p{p} s{s} probability {} > 7",
                            step.probability
                        );
                        assert!(
                            step.note_mask <= 63,
                            "{name} midi t{t} p{p} s{s} note_mask {} > 63",
                            step.note_mask
                        );
                        checked += 1;
                    }
                }
            }
            assert_eq!(checked, 512, "{name} expected exactly 512 midi steps");
        }
    }

    /// Headline regression guard for the note_mask/probability un-swap. Deep.ncs
    /// synth track0 pattern0 step0 has note_mask 0x0f (bits 0..3), probability 7,
    /// and note numbers starting [65, 68, 72, 70, ...]. `active_notes()` yields
    /// exactly popcount(0x0f) == 4 notes. If the two stepInfo bytes are ever
    /// re-swapped, note_mask would read 7 (probability's value) and probability
    /// would read 15 (out of range), reddening the mask, probability, AND count
    /// assertions at once.
    #[test]
    fn synth_stepinfo_semantics_unswapped() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        let step = &deep.synth.tracks[0].patterns[0].steps[0];

        assert_eq!(step.note_mask, 0x0f, "synth t0 p0 s0 note_mask");
        assert_eq!(step.probability, 7, "synth t0 p0 s0 probability");

        assert_eq!(step.notes[0].note_number, 65, "note slot 0 number");
        assert_eq!(step.notes[1].note_number, 68, "note slot 1 number");
        assert_eq!(step.notes[2].note_number, 72, "note slot 2 number");
        assert_eq!(step.notes[3].note_number, 70, "note slot 3 number");

        assert_eq!(
            step.active_notes().count(),
            4,
            "active_notes() must equal popcount(0x0f) == 4"
        );
        assert_eq!(
            step.active_notes().count(),
            step.note_mask.count_ones() as usize,
            "active_notes() count must track note_mask popcount"
        );
    }

    /// The invariant that PROVED which byte is the mask: for every synth step in
    /// Deep.ncs, popcount(note_mask) equals the number of present notes (note
    /// slots with a non-zero note_number). If probability were read as the mask,
    /// its 0..7 values could not track the present-note counts, so this reddens.
    #[test]
    fn mask_bitcount_equals_present_notes() {
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        for (t, track) in deep.synth.tracks.iter().enumerate() {
            for (p, pat) in track.patterns.iter().enumerate() {
                for (s, step) in pat.steps.iter().enumerate() {
                    let present =
                        step.notes.iter().filter(|n| n.note_number != 0).count();
                    assert_eq!(
                        step.note_mask.count_ones() as usize,
                        present,
                        "synth t{t} p{p} s{s}: popcount(note_mask={:#04x}) != present notes {}",
                        step.note_mask,
                        present
                    );
                }
            }
        }
    }

    /// The shared validator now covers the MIDI block. An out-of-range MIDI
    /// note_mask (64 > 63) and, separately, an out-of-range MIDI probability
    /// (8 > 7) are each reported with a message that identifies both the MIDI
    /// plane and the offending field -- proving validate() routes the midi block
    /// through validate_melodic with the "midi" kind, not just the synth block.
    #[test]
    fn midi_validation_reports_bad_values() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.midi.tracks[0].patterns[0].steps[0].note_mask = 64;
        let mask = sess.validate();
        assert!(
            mask.iter().any(|m| m.contains("midi") && m.contains("note_mask")),
            "midi note_mask=64 (> 63) must be reported naming 'midi' and 'note_mask', got {:?}",
            mask
        );

        // Reset the mask to a clean value; only probability is now out of range.
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.midi.tracks[0].patterns[0].steps[0].probability = 8;
        let prob = sess.validate();
        assert!(
            prob.iter().any(|m| m.contains("midi") && m.contains("probability")),
            "midi probability=8 (> 7) must be reported naming 'midi' and 'probability', got {:?}",
            prob
        );
    }

    /// Both real files, whose MIDI blocks are now range-checked alongside the
    /// synth block, still validate clean. Guards against the added midi coverage
    /// introducing a spurious violation on genuinely valid sessions.
    #[test]
    fn real_files_validate_clean_with_midi() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let violations = session.validate();
            assert!(
                violations.is_empty(),
                "{name} is a real valid session but validate() reported {} violation(s): {:?}",
                violations.len(),
                violations
            );
        }
    }

    /// The reverse-engineered per-pattern tail lands in the validator's ranges
    /// across ALL 4*8 = 32 drum patterns of BOTH real files: playback_start &
    /// playback_end <= 31, sync_rate <= 7, playback_direction <= 3. These windows
    /// are tight, so a wrong DRUM_TAIL offset or a stride regression that reads
    /// neighbouring bytes would almost certainly overflow one of them and redden
    /// here. Also pins the observed playback_start alphabet {15, 31} (empirically
    /// confirmed on both samples) -- a shifted base reading a different byte would
    /// break the set, not just the range.
    #[test]
    fn drum_tail_ranges_hold() {
        use std::collections::BTreeSet;
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let mut checked = 0usize;
            let mut starts = BTreeSet::new();
            for (t, track) in session.drums.tracks.iter().enumerate() {
                for (p, pat) in track.patterns.iter().enumerate() {
                    assert!(
                        pat.playback_start <= 31,
                        "{name} drum t{t} p{p} playback_start {} > 31",
                        pat.playback_start
                    );
                    assert!(
                        pat.playback_end <= 31,
                        "{name} drum t{t} p{p} playback_end {} > 31",
                        pat.playback_end
                    );
                    assert!(
                        pat.sync_rate <= 7,
                        "{name} drum t{t} p{p} sync_rate {} > 7",
                        pat.sync_rate
                    );
                    assert!(
                        pat.playback_direction <= 3,
                        "{name} drum t{t} p{p} playback_direction {} > 3",
                        pat.playback_direction
                    );
                    starts.insert(pat.playback_start);
                    checked += 1;
                }
            }
            assert_eq!(checked, 32, "{name} expected exactly 4*8 = 32 drum patterns");
            let observed: Vec<u8> = starts.iter().copied().collect();
            assert!(
                starts.iter().all(|&s| s == 15 || s == 31),
                "{name} playback_start alphabet drifted from the confirmed {{15, 31}}: {:?}",
                observed
            );
        }
    }

    /// Every drum pattern in both files carries exactly AUTOMATION_LANES lanes of
    /// AUTOMATION_LANE_LEN bytes (8 x 192). In Deep.ncs every automation byte is
    /// the 0xFF "unused" sentinel across all lanes and all values -- if the
    /// automation base/stride were wrong, the block would spill into decoded step
    /// data and this all-0xFF invariant would break. Funk.ncs deliberately is NOT
    /// asserted all-0xFF (it carries some live automation values).
    #[test]
    fn drum_automation_shape() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            for (t, track) in session.drums.tracks.iter().enumerate() {
                for (p, pat) in track.patterns.iter().enumerate() {
                    assert_eq!(
                        pat.automation.len(),
                        AUTOMATION_LANES,
                        "{name} drum t{t} p{p} lane count"
                    );
                    assert_eq!(pat.automation.len(), 8, "{name} drum t{t} p{p} lane count literal");
                    for (l, lane) in pat.automation.iter().enumerate() {
                        assert_eq!(
                            lane.len(),
                            AUTOMATION_LANE_LEN,
                            "{name} drum t{t} p{p} lane{l} length"
                        );
                        assert_eq!(lane.len(), 192, "{name} drum t{t} p{p} lane{l} length literal");
                    }
                }
            }
        }

        // Deep.ncs: the entire automation region is the 0xFF unused sentinel.
        let deep = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        for (t, track) in deep.drums.tracks.iter().enumerate() {
            for (p, pat) in track.patterns.iter().enumerate() {
                for (l, lane) in pat.automation.iter().enumerate() {
                    assert!(
                        lane.iter().all(|&b| b == 0xFF),
                        "Deep drum t{t} p{p} lane{l} expected all 0xFF sentinel, got {:?}",
                        lane
                    );
                }
            }
        }
    }

    /// validate() routes the drum per-pattern tail through its range checks: an
    /// out-of-range sync_rate (8 > 7) and, on a fresh clean session, an
    /// out-of-range playback_start (32 > 31) are each reported with a message
    /// naming both the drum plane and the offending field. Re-parsing between the
    /// two mutations keeps them independent, so each assertion proves its own
    /// field is wired -- not that some other tail check fired.
    #[test]
    fn drum_tail_validation_reports_bad() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.drums.tracks[0].patterns[0].sync_rate = 8;
        let sync = sess.validate();
        assert!(
            sync.iter().any(|m| m.contains("drum") && m.contains("sync_rate")),
            "drum sync_rate=8 (> 7) must be reported naming 'drum' and 'sync_rate', got {:?}",
            sync
        );

        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.drums.tracks[0].patterns[0].playback_start = 32;
        let start = sess.validate();
        assert!(
            start.iter().any(|m| m.contains("drum") && m.contains("playback_start")),
            "drum playback_start=32 (> 31) must be reported naming 'drum' and 'playback_start', got {:?}",
            start
        );
    }

    /// The still-undecoded gap (+132..=+167, no validator yet) is carried as a
    /// fixed 36-byte block for round-trip fidelity. Beyond documenting the width
    /// (36 == 167-132+1), prove the carried bytes are the actual file bytes at the
    /// gap offset for track0/pattern0 (velocity-plane base == DRUM_VELOCITY,
    /// pat_base == 0) -- an off-by-one in the gap loop or a shifted base would read
    /// the wrong 36 bytes and redden here.
    #[test]
    fn unknown_gap_is_36_bytes() {
        let raw = load("Deep.ncs");
        let deep = Session::parse(&raw).expect("Deep.ncs must parse");
        let pat = &deep.drums.tracks[0].patterns[0];

        assert_eq!(pat.unknown_132_167.len(), 36, "unknown gap must carry 36 bytes");
        assert_eq!(167 - 132 + 1, 36, "gap +132..=+167 spans 36 bytes");

        let gap_off = DRUM_VELOCITY + 132;
        assert_eq!(
            &pat.unknown_132_167[..],
            &raw[gap_off..gap_off + 36],
            "unknown_132_167 must carry the raw file bytes at +132..=+167"
        );
    }

    /// Regression guard for the newly-added drum-tail range checks: both real,
    /// valid files must still validate() clean. If a drum-tail bound is decoded
    /// wrong (or a check is too strict), one of these genuinely valid sessions
    /// would report a spurious violation here.
    #[test]
    fn real_files_validate_clean_still() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let violations = session.validate();
            assert!(
                violations.is_empty(),
                "{name} is a real valid session but validate() reported {} violation(s): {:?}",
                violations.len(),
                violations
            );
        }
    }

    /// The reverse-engineered melodic per-pattern tail lands in the validator's
    /// ranges across ALL 2*8 = 16 synth AND 16 midi patterns of BOTH real files:
    /// playback_start & playback_end <= 31, sync_rate <= 7, playback_direction <=
    /// 3. These windows are tight, so a wrong MELODIC_TAIL offset or a stride
    /// regression that reads neighbouring step/automation bytes would almost
    /// certainly overflow one of them and redden here. Counting to exactly 16 per
    /// block also pins the 2 tracks x 8 patterns geometry.
    #[test]
    fn melodic_tail_ranges_hold() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            for (kind, tracks) in
                [("synth", &session.synth.tracks), ("midi", &session.midi.tracks)]
            {
                let mut checked = 0usize;
                for (t, track) in tracks.iter().enumerate() {
                    for (p, pat) in track.patterns.iter().enumerate() {
                        assert!(
                            pat.playback_start <= 31,
                            "{name} {kind} t{t} p{p} playback_start {} > 31",
                            pat.playback_start
                        );
                        assert!(
                            pat.playback_end <= 31,
                            "{name} {kind} t{t} p{p} playback_end {} > 31",
                            pat.playback_end
                        );
                        assert!(
                            pat.sync_rate <= 7,
                            "{name} {kind} t{t} p{p} sync_rate {} > 7",
                            pat.sync_rate
                        );
                        assert!(
                            pat.playback_direction <= 3,
                            "{name} {kind} t{t} p{p} playback_direction {} > 3",
                            pat.playback_direction
                        );
                        checked += 1;
                    }
                }
                assert_eq!(
                    checked, 16,
                    "{name} {kind} expected exactly 2*8 = 16 melodic patterns"
                );
            }
        }
    }

    /// Every synth AND midi pattern in both files carries exactly
    /// MELODIC_AUTOMATION_LANES lanes of MELODIC_AUTOMATION_LANE_LEN bytes
    /// (12 x 192). The lane COUNT is the load-bearing distinction from the drum
    /// block, which carries only 8 lanes -- so the literal `== 12` here (NOT 8)
    /// pins the melodic-vs-drum geometry. A wrong lane count or automation stride
    /// would spill the region into neighbouring pattern data.
    #[test]
    fn melodic_automation_is_12_lanes() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            for (kind, tracks) in
                [("synth", &session.synth.tracks), ("midi", &session.midi.tracks)]
            {
                for (t, track) in tracks.iter().enumerate() {
                    for (p, pat) in track.patterns.iter().enumerate() {
                        assert_eq!(
                            pat.automation.len(),
                            MELODIC_AUTOMATION_LANES,
                            "{name} {kind} t{t} p{p} lane count"
                        );
                        assert_eq!(
                            pat.automation.len(),
                            12,
                            "{name} {kind} t{t} p{p} lane count literal (12, NOT the drum block's 8)"
                        );
                        for (l, lane) in pat.automation.iter().enumerate() {
                            assert_eq!(
                                lane.len(),
                                MELODIC_AUTOMATION_LANE_LEN,
                                "{name} {kind} t{t} p{p} lane{l} length"
                            );
                            assert_eq!(
                                lane.len(),
                                192,
                                "{name} {kind} t{t} p{p} lane{l} length literal"
                            );
                        }
                    }
                }
            }
        }
    }

    /// validate() routes the melodic per-pattern tail through its range checks for
    /// BOTH the synth and midi blocks: an out-of-range synth sync_rate (8 > 7) is
    /// reported naming 'synth' and 'sync_rate', and -- on a fresh clean session --
    /// an out-of-range midi playback_start (32 > 31) is reported naming 'midi' and
    /// 'playback_start'. Re-parsing between the two mutations keeps them
    /// independent, so each assertion proves its own block+field is wired, not
    /// that some other tail check fired. The synth/midi split proves both blocks
    /// route through validate_melodic with the correct kind prefix.
    #[test]
    fn melodic_tail_validation_reports_bad() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.synth.tracks[0].patterns[0].sync_rate = 8;
        let sync = sess.validate();
        assert!(
            sync.iter().any(|m| m.contains("synth") && m.contains("sync_rate")),
            "synth sync_rate=8 (> 7) must be reported naming 'synth' and 'sync_rate', got {:?}",
            sync
        );

        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.midi.tracks[0].patterns[0].playback_start = 32;
        let start = sess.validate();
        assert!(
            start.iter().any(|m| m.contains("midi") && m.contains("playback_start")),
            "midi playback_start=32 (> 31) must be reported naming 'midi' and 'playback_start', got {:?}",
            start
        );
    }

    /// The still-undecoded gap (+900..=+935, no validator yet) is carried as a
    /// fixed 36-byte block for round-trip fidelity. Beyond documenting the width
    /// (36 == 935-900+1), prove the carried bytes are the actual file bytes at the
    /// gap offset for synth track0/pattern0 (stepInfo base == SYNTH_BASE, so
    /// pat_base == SYNTH_BASE) -- an off-by-one in the gap loop or a shifted base
    /// would read the wrong 36 bytes and redden here.
    #[test]
    fn melodic_unknown_gap_is_36() {
        let raw = load("Deep.ncs");
        let deep = Session::parse(&raw).expect("Deep.ncs must parse");
        let pat = &deep.synth.tracks[0].patterns[0];

        assert_eq!(pat.unknown_900_935.len(), 36, "unknown gap must carry 36 bytes");
        assert_eq!(935 - 900 + 1, 36, "gap +900..=+935 spans 36 bytes");

        let gap_off = SYNTH_BASE + 900;
        assert_eq!(
            &pat.unknown_900_935[..],
            &raw[gap_off..gap_off + 36],
            "unknown_900_935 must carry the raw file bytes at +900..=+935"
        );
    }

    /// Regression guard for the newly-added melodic-tail range checks: both real,
    /// valid files must still validate() clean. If a melodic-tail bound is decoded
    /// wrong (or a check is too strict), one of these genuinely valid sessions
    /// would report a spurious violation here.
    #[test]
    fn real_files_validate_clean_with_tails() {
        for name in ["Deep.ncs", "Funk.ncs"] {
            let session = Session::parse(&load(name)).expect("sample must parse");
            let violations = session.validate();
            assert!(
                violations.is_empty(),
                "{name} is a real valid session but validate() reported {} violation(s): {:?}",
                violations.len(),
                violations
            );
        }
    }
}
