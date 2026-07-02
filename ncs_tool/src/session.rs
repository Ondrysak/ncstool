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

pub const SYNTH_BASE: usize = 0x2E4;
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

/// A synth/midi step: stepInfo (probability + assignedNoteMask) plus up to 6 notes.
#[derive(Debug, Clone)]
pub struct MelodicStep {
    pub probability: u8,      // VERIFIED 0..63
    pub note_mask: u8,        // VERIFIED 0..7 (bit N => note slot N active)
    pub notes: [Note; NOTES_PER_STEP],
}

impl MelodicStep {
    fn parse(d: &[u8], step_base: usize) -> io::Result<Self> {
        let probability = u8_at(d, step_base + 740)?;
        let note_mask = u8_at(d, step_base + 741)?;
        let mut notes = [Note { note_number: 0, gate: 0, delay: 0, velocity: 0 }; NOTES_PER_STEP];
        for (n, note) in notes.iter_mut().enumerate() {
            let b = step_base + 744 + n * 4;
            *note = Note {
                note_number: u8_at(d, b)?,
                gate: u8_at(d, b + 1)?,
                delay: u8_at(d, b + 2)?,
                velocity: u8_at(d, b + 3)?,
            };
        }
        Ok(MelodicStep { probability, note_mask, notes })
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

#[derive(Debug, Clone)]
pub struct MelodicPattern {
    pub steps: Vec<MelodicStep>, // 32
}

#[derive(Debug, Clone)]
pub struct SynthTrack {
    pub patterns: Vec<MelodicPattern>, // 8
}

#[derive(Debug, Clone)]
pub struct SynthData {
    pub tracks: Vec<SynthTrack>, // 2
}

impl SynthData {
    pub const TRACKS: usize = 2;
    pub const PATTERNS: usize = 8;
    pub const STEPS: usize = 32;

    pub fn parse(d: &[u8]) -> io::Result<Self> {
        let mut tracks = Vec::with_capacity(Self::TRACKS);
        for t in 0..Self::TRACKS {
            let mut patterns = Vec::with_capacity(Self::PATTERNS);
            for p in 0..Self::PATTERNS {
                let mut steps = Vec::with_capacity(Self::STEPS);
                for s in 0..Self::STEPS {
                    let step_base =
                        t * SYNTH_TRACK_STRIDE + p * PATTERN_STRIDE_SYNTH + s * STEP_STRIDE;
                    steps.push(MelodicStep::parse(d, step_base)?);
                }
                patterns.push(MelodicPattern { steps });
            }
            tracks.push(SynthTrack { patterns });
        }
        Ok(SynthData { tracks })
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

#[derive(Debug, Clone)]
pub struct DrumPattern {
    pub steps: Vec<DrumStep>, // 32
}

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
                let mut steps = Vec::with_capacity(Self::STEPS);
                for s in 0..Self::STEPS {
                    let idx = t * DRUM_TRACK_STRIDE + p * PATTERN_STRIDE_DRUM + s;
                    steps.push(DrumStep {
                        velocity: u8_at(d, DRUM_VELOCITY + idx)?,
                        probability: u8_at(d, DRUM_PROBABILITY + idx)?,
                        choice: u8_at(d, DRUM_CHOICE + idx)?,
                        rhythm: u8_at(d, DRUM_RHYTHM + idx)?,
                    });
                }
                patterns.push(DrumPattern { steps });
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
    pub drums: DrumData,
    pub scale: Scale,
    pub fx: Fx,
    // pending: header, scenes, chains, synth/drum/midi tail + automation, midi patterns, track info, octaves
}

impl Session {
    pub fn parse(d: &[u8]) -> io::Result<Self> {
        if d.len() != FILE_SIZE {
            return Err(err("not a Circuit Tracks .ncs (expected 160780 bytes)"));
        }
        Ok(Session {
            timing: Timing::parse(d)?,
            synth: SynthData::parse(d)?,
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
        for (ti, track) in self.synth.tracks.iter().enumerate() {
            for (pi, pat) in track.patterns.iter().enumerate() {
                for (si, step) in pat.steps.iter().enumerate() {
                    if step.probability > 63 {
                        v.push(format!(
                            "synth[{}][{}].step[{}].probability {} > 63", ti, pi, si, step.probability));
                    }
                    if step.note_mask > 7 {
                        v.push(format!(
                            "synth[{}][{}].step[{}].note_mask {} > 7", ti, pi, si, step.note_mask));
                    }
                    for (ni, note) in step.notes.iter().enumerate() {
                        if (step.note_mask >> ni) & 1 == 1 {
                            if note.gate > 127 {
                                v.push(format!("synth[{}][{}].step[{}].note[{}].gate {} > 127", ti, pi, si, ni, note.gate));
                            }
                            if note.velocity > 127 {
                                v.push(format!("synth[{}][{}].step[{}].note[{}].velocity {} > 127", ti, pi, si, ni, note.velocity));
                            }
                            if note.note_number != 0 && !(1..=139).contains(&note.note_number) {
                                v.push(format!("synth[{}][{}].step[{}].note[{}].note_number {} not 0 or 1..=139", ti, pi, si, ni, note.note_number));
                            }
                        }
                    }
                }
            }
        }
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
                            step.probability <= 63,
                            "{name} t{t} p{p} s{s} probability {} > 63",
                            step.probability
                        );
                        assert!(
                            step.note_mask <= 7,
                            "{name} t{t} p{p} s{s} note_mask {} > 7",
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

    /// A synth step probability past 63 is flagged, and the message names both
    /// the field and the limit it violated.
    #[test]
    fn bad_synth_probability_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.synth.tracks[0].patterns[0].steps[0].probability = 200;

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("probability") && m.contains("63")),
            "probability=200 must be reported naming 'probability' and the '63' limit, got {:?}",
            v
        );
    }

    /// A synth step note_mask past 7 (only 3 bits are valid) is flagged with a
    /// message naming the field.
    #[test]
    fn bad_note_mask_reported() {
        let mut sess = Session::parse(&load("Deep.ncs")).expect("Deep.ncs must parse");
        sess.synth.tracks[0].patterns[0].steps[0].note_mask = 8;

        let v = sess.validate();
        assert!(
            v.iter().any(|m| m.contains("note_mask")),
            "note_mask=8 (> 7) must be reported naming note_mask, got {:?}",
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
}
