use std::fs::File;
use std::io::{self, Read};

mod kaitai;
mod session;

const TRACKS: usize = 4;
const PATTERNS: usize = 8;
const STEPS: usize = 32;










/// Drum-plane write offsets for the `clone` editor. Kept here (not in `.ksy`)
/// because the Kaitai runtime is read-only — writing edits back needs explicit
/// offsets. Mirrors decompiled_validators/ncs.ksy `drums`.
#[derive(Debug, Clone)]
struct Offsets {
    velocity: usize,
    probability: usize,
    track_stride: usize,
    pattern_stride: usize,
}

fn read_file(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}


fn step_symbol(velocity: u8, probability: u8) -> String {
    if velocity == 0 {
        return ".".into();
    }
    const LEVELS: &[char] = &['▁', '▃', '▅', '█'];
    let idx = ((velocity as usize * LEVELS.len()) / 128).min(LEVELS.len() - 1);
    let ch = LEVELS[idx];
    // Append a single probability digit similar to the Python/TUI helpers
    format!("{}{}", ch, probability % 10)
}

/// ASCII render for typed session drum steps (mirrors render_ascii but on session types).
fn render_drum_ascii(steps: &[session::DrumStep]) -> String {
    let mut out = String::new();
    for (i, st) in steps.iter().enumerate() {
        if i > 0 {
            if i % 8 == 0 { out.push('\n'); } else { out.push(' '); }
        }
        out.push_str(&step_symbol(st.velocity, st.probability));
    }
    out
}

/// Compact summary of a synth pattern: active step count + the notes on its first active step.
fn synth_pattern_summary(pattern: &session::MelodicPattern) -> String {
    let active: Vec<&session::MelodicStep> =
        pattern.steps.iter().filter(|s| s.note_mask != 0).collect();
    if active.is_empty() {
        return "(empty)".to_string();
    }
    let first = active[0];
    let notes: Vec<String> = first
        .active_notes()
        .filter(|n| n.is_present())
        .map(|n| format!("{}", n.note_number))
        .collect();
    format!(
        "{} active step(s); first: notes[{}] vel {} prob {}",
        active.len(),
        notes.join(","),
        first.active_notes().next().map(|n| n.velocity).unwrap_or(0),
        first.probability
    )
}


/// Velocity levels for step-character digits `0`..`9` (from README pattern format).
const VEL_LEVELS: [u8; 10] = [0, 14, 28, 42, 56, 70, 84, 98, 112, 127];

/// Default drum-array offsets observed across multiple Circuit Tracks packs.
fn default_drum_offsets() -> Offsets {
    Offsets {
        velocity: 0x0CD74,
        probability: 0x0CD94,
        track_stride: 0x3540,
        pattern_stride: 0x06A8,
    }
}

/// Map a single step character to a velocity (README pattern format).
fn step_char_to_velocity(c: char) -> io::Result<u8> {
    match c {
        'X' => Ok(127),
        'x' => Ok(32),
        '.' => Ok(0),
        '0'..='9' => Ok(VEL_LEVELS[(c as u8 - b'0') as usize]),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid step char '{}': expected 'X', 'x', '.', or '0'-'9'", c),
        )),
    }
}

/// A parsed `track:pattern:steps[:probability]` pattern edit.
#[derive(Debug, Clone)]
struct PatternEdit {
    track: usize,
    pattern: usize,
    velocities: Vec<u8>, // one entry per specified step (<= STEPS)
    probability: Option<u8>,
}

fn invalid_data(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

fn parse_index(s: &str, limit: usize, what: &str) -> io::Result<usize> {
    let v: usize = s
        .trim()
        .parse()
        .map_err(|_| invalid_data(format!("invalid {} '{}': expected 0..{}", what, s, limit - 1)))?;
    if v >= limit {
        return Err(invalid_data(format!("{} {} out of range 0..{}", what, v, limit - 1)));
    }
    Ok(v)
}

fn parse_pattern_edit(spec: &str) -> io::Result<PatternEdit> {
    let parts: Vec<&str> = spec.splitn(4, ':').collect();
    if parts.len() < 3 {
        return Err(invalid_data(format!(
            "pattern spec '{}' must be track:pattern:steps[:probability]",
            spec
        )));
    }
    let track = parse_index(parts[0], TRACKS, "track")?;
    let pattern = parse_index(parts[1], PATTERNS, "pattern")?;

    let steps_str = parts[2];
    let step_count = steps_str.chars().count();
    if step_count == 0 {
        return Err(invalid_data(format!("pattern spec '{}' has no steps", spec)));
    }
    if step_count > STEPS {
        return Err(invalid_data(format!(
            "too many steps: {} (max {})",
            step_count, STEPS
        )));
    }
    let velocities = steps_str
        .chars()
        .map(step_char_to_velocity)
        .collect::<io::Result<Vec<u8>>>()?;

    let probability = match parts.get(3) {
        Some(p) => {
            let v: u8 = p
                .trim()
                .parse()
                .map_err(|_| invalid_data(format!("invalid probability '{}': expected 0..9", p)))?;
            if v > 9 {
                return Err(invalid_data(format!("probability {} out of range 0..9", v)));
            }
            Some(v)
        }
        None => None,
    };

    Ok(PatternEdit { track, pattern, velocities, probability })
}

/// Write an edit's velocity + probability planes into `data`.
/// Steps beyond the edit's length are left untouched (edit only the given steps).
fn apply_pattern_edit(data: &mut [u8], off: &Offsets, edit: &PatternEdit) -> io::Result<()> {
    let base = edit.track * off.track_stride + edit.pattern * off.pattern_stride;
    let prob = edit.probability.unwrap_or(7); // full-probability default for played hits
    for (s, &vel) in edit.velocities.iter().enumerate() {
        let idx = base + s;
        let v_off = off.velocity + idx;
        let p_off = off.probability + idx;
        if v_off >= data.len() || p_off >= data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "edit offset out of bounds (track {}, pattern {}, step {})",
                    edit.track, edit.pattern, s
                ),
            ));
        }
        data[v_off] = vel;
        data[p_off] = if vel > 0 { prob } else { 0 };
    }
    Ok(())
}

/// Clone `source` into `target`, applying one or more pattern edits.
fn run_clone(source: &str, target: &str, specs: &[String]) -> io::Result<()> {
    let mut data = read_file(source)?;
    let off = default_drum_offsets();

    // Parse everything up front so a bad spec fails before any write.
    let edits = specs
        .iter()
        .map(|s| parse_pattern_edit(s))
        .collect::<io::Result<Vec<_>>>()?;

    for edit in &edits {
        apply_pattern_edit(&mut data, &off, edit)?;
    }

    // Gate the edit through the typed model: parse the mutated buffer and run a
    // verified subset of the validator's range checks. Refuse to write on failure.
    // (Mirrors only the validators we've confirmed, not the full device validator.)
    let sess = session::Session::parse(&data)?;
    let violations = sess.validate();
    if !violations.is_empty() {
        eprintln!("[error] edited session fails typed validation subset ({} issue(s)):", violations.len());
        for msg in violations.iter().take(10) {
            eprintln!("  - {}", msg);
        }
        return Err(invalid_data("edited session failed the typed validation subset".into()));
    }

    std::fs::write(target, &data)?;

    println!("Cloned {} -> {} with {} pattern edit(s):", source, target, edits.len());
    for edit in &edits {
        let prob = edit
            .probability
            .map(|p| format!(" prob={}", p))
            .unwrap_or_default();
        println!(
            "  track {} pattern {}: {} step(s){}",
            edit.track,
            edit.pattern,
            edit.velocities.len(),
            prob
        );
    }
    Ok(())
}

fn run_analyze(file_path: &str) -> io::Result<()> {
    let data = read_file(file_path)?;

    // Typed model (validator-derived) for timing / synth / drums / scale / fx.
    let sess = session::Session::parse(&data)?;

    // Coverage metric, computed from the typed model (all sections now typed).
    let melodic_pat = 32 * 28 + 2 + 1 + 1 + 12 * 192; // steps + tail (gap +900..935 excluded)
    let drum_pat = 4 * 32 + 2 + 1 + 1 + 8 * 192;       // planes + tail (gap +132..167 excluded)
    let known = 3 + 8                       // timing bytes + spare dwords
        + (16 * 8 * 4) + 4 + (8 * 4)        // scenes + scene_chain + pattern_chains
        + (2 * (2 * 8 * melodic_pat))       // synth + midi patterns
        + (4 * 8 * drum_pat)                // drum patterns
        + 4                                 // header file_size
        + (2 * 3) + (2 * 3)                 // synth+midi track_info (3 stored bytes/track)
        + 4 + 4 + 2;                        // drum mutes + choices + octaves
    let total = data.len();
    println!(
        "Parsed/carried bytes: {} / {} ({:.2}%) | fully typed via Kaitai: timing, scenes+chains, synth+midi(steps+tail)+track_info, drums(+tail)+mutes+choices, scale, fx, octaves, header; automation carried RAW; per-pattern 36B gaps + header feature-flags not counted",
        known, total, (known as f64) * 100.0 / (total.max(1) as f64)
    );

    // ---- typed header ----
    let t = &sess.timing;
    println!("Timing: tempo={} swing={} swing_sync_rate={} spare1={} spare2={}",
             t.tempo, t.swing, t.swing_sync_rate, t.spare1, t.spare2);
    println!("Scale: root={} type={}", sess.scale.root, sess.scale.scale_type);
    println!("FX: delay_preset={} reverb_preset={}", sess.fx.delay_preset, sess.fx.reverb_preset);
    println!("SynthTrackInfo: {:?}", sess.synth_track_info.iter()
        .map(|t| (t.patch, t.mute_state, t.sidechain_preset)).collect::<Vec<_>>());
    println!("MidiTrackInfo:  {:?}", sess.midi_track_info.iter()
        .map(|t| (t.patch, t.mute_state, t.sidechain_preset)).collect::<Vec<_>>());
    println!("DrumMuteStates: {:?}  DefaultDrumChoices: {:?}  MidiOctaves: {:?}",
             sess.drum_mute_states, sess.default_drum_choices, sess.midi_keyboard_octaves);

    // ---- scenes & chains (typed) ----
    let sc = &sess.scene_chain;
    println!("Scenes: 16x8 typed | SceneChain: {}..{} (pad {}) | PatternChains: {} entries",
             sc.start, sc.end, sc.pad, sess.pattern_chains.len());

    // ---- synth tracks (NEW: typed) ----
    for (ti, track) in sess.synth.tracks.iter().enumerate() {
        println!("\n=== SYNTH TRACK {} ===", ti);
        for (pi, pat) in track.patterns.iter().enumerate() {
            println!("P{:02}: {}", pi, synth_pattern_summary(pat));
        }
    }

    // ---- midi tracks (NEW: typed, same shape as synth) ----
    for (ti, track) in sess.midi.tracks.iter().enumerate() {
        println!("\n=== MIDI TRACK {} ===", ti);
        for (pi, pat) in track.patterns.iter().enumerate() {
            println!("P{:02}: {}", pi, synth_pattern_summary(pat));
        }
    }

    // ---- drums (typed, ASCII) ----
    for (ti, track) in sess.drums.tracks.iter().enumerate() {
        println!("\n=== DRUM TRACK {} ===", ti);
        for (pi, pat) in track.patterns.iter().enumerate() {
            let ascii = render_drum_ascii(&pat.steps);
            let label = format!("P{:02}: ", pi);
            let mut lines = ascii.lines();
            if let Some(first) = lines.next() {
                println!("{}{}", label, first);
                let pad = " ".repeat(label.len());
                for line in lines { println!("{}{}", pad, line); }
            } else {
                println!("{}", label);
            }
        }
    }

    Ok(())
}

fn print_usage(prog: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <file.ncs>                                  analyze a session", prog);
    eprintln!("  {} clone <src.ncs> <dst.ncs> \"t:p:steps[:prob]\" ...  edit drum patterns", prog);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(|s| s.as_str()).unwrap_or("ncs-tui");

    match args.get(1).map(|s| s.as_str()) {
        Some("clone") => {
            if args.len() < 5 {
                print_usage(prog);
                std::process::exit(2);
            }
            run_clone(&args[2], &args[3], &args[4..])
        }
        Some(file) => run_analyze(file),
        None => {
            print_usage(prog);
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_drums(path: &str) -> session::DrumData {
        let data = read_file(path).expect("failed to read test ncs file");
        session::Session::parse(&data).expect("parse session").drums
    }

    #[test]
    fn deep_track2_pattern02_structure() {
        let drums = load_drums("../test_data/Deep.ncs");
        let steps = &drums.tracks[2].patterns[2].steps;
        // First bar (0..7): █ ▁ . ▁ █ ▁ . ▁
        assert!(steps[0].velocity >= 96, "step0 expected strong hit");
        assert!(steps[1].velocity > 0 && steps[1].velocity <= 31, "step1 expected weak hit");
        assert_eq!(steps[2].velocity, 0, "step2 expected rest");
        assert!(steps[3].velocity > 0 && steps[3].velocity <= 31, "step3 expected weak hit");
        assert!(steps[4].velocity >= 96, "step4 expected strong hit");
        assert!(steps[5].velocity > 0 && steps[5].velocity <= 31, "step5 expected weak hit");
        assert_eq!(steps[6].velocity, 0, "step6 expected rest");
        assert!(steps[7].velocity > 0 && steps[7].velocity <= 31, "step7 expected weak hit");
        // Probability commonly 7 for these datasets
        for &i in &[0,1,3,4,5,7] { assert_eq!(steps[i].probability, 7, "prob mismatch at step {}", i); }
    }

    #[test]
    fn funk_track2_pattern02_structure() {
        let drums = load_drums("../test_data/Funk.ncs");
        let steps = &drums.tracks[2].patterns[2].steps;
        // First bar (0..7): █ ▁ ▁ ▁ █ ▁ ▁ ▁
        assert!(steps[0].velocity >= 96, "step0 expected strong hit");
        for &i in &[1usize,2,3,5,6,7] {
            assert!(steps[i].velocity > 0 && steps[i].velocity <= 31, "step{} expected weak hit", i);
        }
        assert!(steps[4].velocity >= 96, "step4 expected strong hit");
        // Bars 3 and 4 (16..31) were rests in Funk P02
        for i in 16..32 { assert_eq!(steps[i].velocity, 0, "expected rest at step {}", i); }
    }

    #[test]
    fn step_char_to_velocity_maps_symbols_and_digits() {
        assert_eq!(step_char_to_velocity('X').unwrap(), 127);
        assert_eq!(step_char_to_velocity('x').unwrap(), 32);
        assert_eq!(step_char_to_velocity('.').unwrap(), 0);
        assert_eq!(step_char_to_velocity('0').unwrap(), 0);
        assert_eq!(step_char_to_velocity('9').unwrap(), 127);
        assert_eq!(step_char_to_velocity('5').unwrap(), 70);
        assert!(step_char_to_velocity('q').is_err(), "unknown char must error");
    }

    #[test]
    fn parse_pattern_edit_happy_paths() {
        let e = parse_pattern_edit("0:0:X...X...X...X...").unwrap();
        assert_eq!(e.track, 0);
        assert_eq!(e.pattern, 0);
        assert_eq!(e.velocities.len(), 16);
        assert_eq!(e.velocities[0], 127);
        assert_eq!(e.velocities[1], 0);
        assert_eq!(e.probability, None);

        let e = parse_pattern_edit("1:2:x.x.:5").unwrap();
        assert_eq!(e.track, 1);
        assert_eq!(e.pattern, 2);
        assert_eq!(e.velocities, vec![32u8, 0, 32, 0]);
        assert_eq!(e.probability, Some(5));
    }

    #[test]
    fn parse_pattern_edit_error_paths() {
        assert!(parse_pattern_edit("0:0").is_err(), "missing steps field");
        assert!(parse_pattern_edit("0:0:").is_err(), "empty steps");
        let too_long = format!("0:0:{}", "X".repeat(33));
        assert!(parse_pattern_edit(&too_long).is_err(), "more than 32 steps");
        assert!(parse_pattern_edit("0:0:Xq..").is_err(), "invalid step char");
        assert!(parse_pattern_edit("4:0:X").is_err(), "track out of range");
        assert!(parse_pattern_edit("0:8:X").is_err(), "pattern out of range");
        assert!(parse_pattern_edit("0:0:X:10").is_err(), "probability out of range");
        assert!(parse_pattern_edit("a:0:X").is_err(), "non-numeric track");
    }

    #[test]
    fn apply_pattern_edit_roundtrips_through_real_data() {
        let mut data = read_file("../test_data/Deep.ncs").expect("read Deep.ncs");
        let orig_len = data.len();
        let offsets = default_drum_offsets();
        let edit = parse_pattern_edit("0:0:X...X...X...X...").unwrap();
        apply_pattern_edit(&mut data, &offsets, &edit).expect("apply edit");
        assert_eq!(data.len(), orig_len, "edit must not resize the buffer");

        let drums = session::Session::parse(&data).expect("parse edited session").drums;
        let steps = &drums.tracks[0].patterns[0].steps;
        assert_eq!(steps[0].probability, 7, "played hit gets default probability 7");
        assert_eq!(steps[1].velocity, 0);
        assert_eq!(steps[1].probability, 0, "rest gets zero probability");
        assert_eq!(steps[4].velocity, 127);
        assert_eq!(steps[8].velocity, 127);
        assert_eq!(steps[12].velocity, 127);
    }

    #[test]
    fn apply_pattern_edit_zeroes_rest_probability_and_honors_custom_prob() {
        let mut data = read_file("../test_data/Deep.ncs").expect("read Deep.ncs");
        let offsets = default_drum_offsets();
        let edit = parse_pattern_edit("1:0:x.x.:5").unwrap();
        apply_pattern_edit(&mut data, &offsets, &edit).expect("apply edit");

        let drums = session::Session::parse(&data).expect("parse edited session").drums;
        let steps = &drums.tracks[1].patterns[0].steps;
        assert_eq!(steps[0].velocity, 32);
        assert_eq!(steps[0].probability, 5, "played hit uses the custom probability");
        assert_eq!(steps[1].velocity, 0);
        assert_eq!(steps[1].probability, 0, "rest probability is forced to zero");
    }

    #[test]
    fn apply_pattern_edit_errors_when_offset_exceeds_buffer() {
        let mut data = vec![0u8; 16];
        let offsets = default_drum_offsets();
        let edit = parse_pattern_edit("0:0:X").unwrap();
        let result = apply_pattern_edit(&mut data, &offsets, &edit);
        assert!(result.is_err(), "velocity offset beyond a tiny buffer must error");
    }
}

