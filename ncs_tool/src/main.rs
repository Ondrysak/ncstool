use std::fs::File;
use std::io::{self, Read};

mod session;

const TRACKS: usize = 4;
const PATTERNS: usize = 8;
const STEPS: usize = 32;

#[derive(Debug, Clone)]
struct Step {
    velocity: u8,
    probability: u8,
    choice: u8,
    mask: u8,
    pitch: u8,
    decay: u8,
    distortion: u8,
    eq: u8,
}

#[derive(Debug, Clone)]
struct Pattern {
    steps: [Step; STEPS],
}

#[derive(Debug, Clone)]
struct DrumTrack {
    patterns: [Pattern; PATTERNS],
}

#[derive(Debug, Clone)]
struct DrumData {
    tracks: [DrumTrack; TRACKS],

}







fn render_ascii_bool(steps: &[bool]) -> String {
    let mut out = String::new();
    for (i, on) in steps.iter().enumerate() {
        if i > 0 {
            if i % 8 == 0 { out.push('\n'); } else { out.push(' '); }
        }
        out.push_str(if *on { "█" } else { "." });
    }
    out
}



#[derive(Debug, Clone)]
struct Fx {
    delay_preset: u8,  // 0..15
    reverb_preset: u8, // 0..7
}

#[derive(Debug, Clone)]
struct FxOffsets {
    delay_preset: usize,
    reverb_preset: usize,
}

impl Fx {
    fn from_bytes(data: &[u8], off: &FxOffsets) -> io::Result<Self> {
        if off.delay_preset >= data.len() || off.reverb_preset >= data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "FX offset out of bounds"));
        }
        let fx = Fx {
            delay_preset: data[off.delay_preset],
            reverb_preset: data[off.reverb_preset],
        };
        // Validate ranges based on validate_fx_presets() from decompiled code
        if fx.delay_preset >= 0x10 {
            eprintln!("[warn] Session's delay preset index is out of range: {} (expected 0..15)", fx.delay_preset);
        }
        if fx.reverb_preset >= 0x08 {
            eprintln!("[warn] Session's reverb preset index is out of range: {} (expected 0..7)", fx.reverb_preset);
        }
        Ok(fx)
    }
}


#[derive(Debug, Clone)]
struct Timing {
    tempo: u8,           // 40..240 BPM (inclusive)
    swing: u8,           // 20..80 (inclusive)
    swing_sync_rate: u8, // 0..7
    spare1: u32,         // must be 0
    spare2: u32,         // must be 0
}

#[derive(Debug, Clone, Copy)]
struct TimingOffsets {
    tempo: usize,           // +0x34
    swing: usize,           // +0x35
    swing_sync_rate: usize, // +0x36
    spare1: usize,          // +0x38 (u32 LE)
    spare2: usize,          // +0x3C (u32 LE)
}

impl Timing {
    fn from_bytes(data: &[u8], off: &TimingOffsets) -> io::Result<Self> {
        // Bounds checks
        for &idx in [off.tempo, off.swing, off.swing_sync_rate].iter() {
            if idx >= data.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Timing byte offset out of bounds"));
            }
        }
        if off.spare1 + 4 > data.len() || off.spare2 + 4 > data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Timing dword offset out of bounds"));
        }
        let tempo = data[off.tempo];
        let swing = data[off.swing];
        let swing_sync_rate = data[off.swing_sync_rate];
        let spare1 = u32::from_le_bytes([data[off.spare1], data[off.spare1 + 1], data[off.spare1 + 2], data[off.spare1 + 3]]);
        let spare2 = u32::from_le_bytes([data[off.spare2], data[off.spare2 + 1], data[off.spare2 + 2], data[off.spare2 + 3]]);
        // Mirror firmware range checks
        if !(40..=240).contains(&tempo) {
            eprintln!("[warn] Tempo out of range: {} (expected 40..240)", tempo);
        }
        if !(20..=80).contains(&swing) {
            eprintln!("[warn] Swing out of range: {} (expected 20..80)", swing);
        }
        if swing_sync_rate >= 8 {
            eprintln!("[warn] Swing sync rate out of range: {} (expected 0..7)", swing_sync_rate);
        }
        if spare1 != 0 {
            eprintln!("[warn] Session timing spare1 not set to zero: {}", spare1);
        }
        if spare2 != 0 {
            eprintln!("[warn] Session timing spare2 not set to zero: {}", spare2);
        }
        Ok(Timing { tempo, swing, swing_sync_rate, spare1, spare2 })
    }
}

#[derive(Debug, Clone, Copy)]
struct SceneEntry { start: u8, end: u8, pad: u16 }

#[derive(Debug, Clone)]
struct Scene { entries: [SceneEntry; 8] }

#[derive(Debug, Clone)]
struct Scenes { scenes: [Scene; 16] }

#[derive(Debug, Clone, Copy)]
struct ScenesOffsets {
    base: usize,          // 0x40
    scene_stride: usize,  // 0x28
    entry_stride: usize,  // 4
}

impl Scenes {
    fn from_bytes(data: &[u8], off: &ScenesOffsets) -> io::Result<Self> {
        let mut scenes: [Scene; 16] = unsafe { std::mem::zeroed() };
        for si in 0..16 {
            let mut entries: [SceneEntry; 8] = unsafe { std::mem::zeroed() };
            for ei in 0..8 {
                let idx = off.base + si * off.scene_stride + ei * off.entry_stride;
                if idx + 4 > data.len() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Scenes offset out of bounds")); }
                let start = data[idx];
                let end = data[idx + 1];
                let pad = u16::from_le_bytes([data[idx + 2], data[idx + 3]]);
                // Mirror key firmware checks
                if start >= 8 { eprintln!("[warn] Scene {} entry {} start out of range: {}", si, ei, start); }
                if end >= 8 { eprintln!("[warn] Scene {} entry {} end out of range: {}", si, ei, end); }
                if end < start { eprintln!("[warn] Scene {} entry {} end < start ({} < {})", si, ei, end, start); }
                if pad != 0 { eprintln!("[warn] Scene {} entry {} padding not zero: {}", si, ei, pad); }
                entries[ei] = SceneEntry { start, end, pad };
            }
            scenes[si] = Scene { entries };
        }
        Ok(Scenes { scenes })
    }
}

#[derive(Debug, Clone, Copy)]
struct SceneChain { start_scene: u8, end_scene: u8, pad: u16 }

#[derive(Debug, Clone, Copy)]
struct PatternChainEntry { start: u8, end: u8, pad: u16 }

#[derive(Debug, Clone)]
struct PatternChains { entries: [PatternChainEntry; 8] }

#[derive(Debug, Clone, Copy)]
struct ChainOffsets {
    scene_chain_base: usize,    // 0x2C0 (start,end,pad u16)
    pattern_chain_base: usize,  // 0x2C4 (array of 8 entries, stride 4)
    pattern_chain_stride: usize // 4
}

impl SceneChain {
    fn from_bytes(data: &[u8], off: &ChainOffsets) -> io::Result<Self> {
        let b = off.scene_chain_base;
        if b + 4 > data.len() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "SceneChain out of bounds")); }
        let start_scene = data[b];
        let end_scene = data[b + 1];
        let pad = u16::from_le_bytes([data[b + 2], data[b + 3]]);
        if start_scene >= 16 { eprintln!("[warn] Scene chain start out of range: {} (expected 0..15)", start_scene); }
        if end_scene >= 16 { eprintln!("[warn] Scene chain end out of range: {} (expected 0..15)", end_scene); }
        if end_scene < start_scene { eprintln!("[warn] Scene chain end < start ({} < {})", end_scene, start_scene); }
        if pad != 0 { eprintln!("[warn] Scene chain padding not set to 0: {}", pad); }
        Ok(SceneChain { start_scene, end_scene, pad })
    }
}

impl PatternChains {
    fn from_bytes(data: &[u8], off: &ChainOffsets) -> io::Result<Self> {
        let mut entries: [PatternChainEntry; 8] = unsafe { std::mem::zeroed() };
        for i in 0..8 {
            let idx = off.pattern_chain_base + i * off.pattern_chain_stride;
            if idx + 4 > data.len() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "PatternChains out of bounds")); }
            let start = data[idx];
            let end = data[idx + 1];
            let pad = u16::from_le_bytes([data[idx + 2], data[idx + 3]]);
            if start >= 8 { eprintln!("[warn] Pattern chain {} start out of range: {} (0..7)", i, start); }
            if end >= 8 { eprintln!("[warn] Pattern chain {} end out of range: {} (0..7)", i, end); }
            if end < start { eprintln!("[warn] Pattern chain {} end < start ({} < {})", i, end, start); }
            if pad != 0 { eprintln!("[warn] Pattern chain {} padding not set to 0: {}", i, pad); }
            entries[i] = PatternChainEntry { start, end, pad };
        }
        Ok(PatternChains { entries })
    }
}

#[derive(Debug, Clone, Copy)]
struct ScaleSettings { root: u8, scale_type: u8 }

#[derive(Debug, Clone, Copy)]
struct ScaleOffsets { root: usize, scale_type: usize }

impl ScaleSettings {
    fn from_bytes(data: &[u8], off: &ScaleOffsets) -> io::Result<Self> {
        if off.root >= data.len() || off.scale_type >= data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Scale offsets out of bounds"));
        }
        let root = data[off.root];
        let scale_type = data[off.scale_type];
        if root >= 12 { eprintln!("[warn] Scale root out of range: {} (expected 0..11)", root); }
        if scale_type >= 16 { eprintln!("[warn] Invalid scale type: {} (expected 0..15)", scale_type); }
        Ok(ScaleSettings { root, scale_type })
    }
}








impl DrumData {
    fn from_bytes(data: &[u8], offsets: &Offsets) -> io::Result<Self> {
        let mut tracks: [DrumTrack; TRACKS] = unsafe { std::mem::zeroed() };

        for t in 0..TRACKS {
            let mut patterns: [Pattern; PATTERNS] = unsafe { std::mem::zeroed() };

            for p in 0..PATTERNS {
                let mut steps: [Step; STEPS] = unsafe { std::mem::zeroed() };

                for s in 0..STEPS {
                    let idx = t * offsets.track_stride + p * offsets.pattern_stride + s;

                    steps[s] = Step {
                        velocity: data[offsets.velocity + idx],
                        probability: data[offsets.probability + idx],
                        choice: data[offsets.choice + idx],
                        mask: data[offsets.mask + idx],
                        pitch: data[offsets.pitch + idx],
                        decay: data[offsets.decay + idx],
                        distortion: data[offsets.distortion + idx],
                        eq: data[offsets.eq + idx],
                    };
                }

                patterns[p] = Pattern { steps };
            }

            tracks[t] = DrumTrack { patterns };
        }

        Ok(DrumData { tracks })
    }
}

#[derive(Debug, Clone)]
struct Offsets {
    velocity: usize,
    probability: usize,
    choice: usize,
    mask: usize,
    pitch: usize,
    decay: usize,
    distortion: usize,
    eq: usize,
    track_stride: usize,
    pattern_stride: usize,
}

fn read_file(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}


// Simple coverage metric: count bytes we can confidently interpret (validated via firmware)
// Currently: per-step velocity/probability/choice/mask (4 planes) + 2 FX preset bytes
fn compute_known_bytes(data: &[u8], off: &Offsets, fx: &FxOffsets) -> usize {
    let mut known: usize = 0;
    // Helper to count per-step plane
    let count_plane = |base: usize| {
        let mut c = 0usize;
        for t in 0..TRACKS {
            for p in 0..PATTERNS {
                for s in 0..STEPS {
                    let idx = base + t * off.track_stride + p * off.pattern_stride + s;
                    if idx < data.len() { c += 1; }
                }
            }
        }
        c
    };
    known += count_plane(off.velocity);
    known += count_plane(off.probability);
    known += count_plane(off.choice);
    known += count_plane(off.mask);
    // FX bytes
    if fx.delay_preset < data.len() { known += 1; }
    if fx.reverb_preset < data.len() { known += 1; }
    known
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

fn render_ascii(steps: &[Step], show_prob: bool) -> String {
    let mut out = String::new();
    for (i, st) in steps.iter().enumerate() {
        if i > 0 {
            if i % 8 == 0 { out.push('\n'); } else { out.push(' '); }
        }
        let sym = if show_prob { step_symbol(st.velocity, st.probability) } else { if st.velocity == 0 { ".".into() } else { "█".into() } };
        out.push_str(&sym);
    }
    out
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
        choice: 0x0CDB4,
        mask: 0x0CDD4,
        pitch: 0x0CDF4,
        decay: 0x0CE14,
        distortion: 0x0CE34,
        eq: 0x0CE54,
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

    // Coverage metric + scenes/chains still use the legacy structs (Session does
    // not model those regions yet). Dual-parse is intentional and temporary.
    let offsets = default_drum_offsets();
    let fx_offsets = FxOffsets { delay_preset: 0x00026D0E, reverb_preset: 0x00026D0F };
    let known = compute_known_bytes(&data, &offsets, &fx_offsets)
        + 3   // timing bytes
        + 8   // timing dwords
        + (16 * 8 * 4)  // scenes table
        + 4             // scene chain
        + (8 * 4)       // pattern chains
        // synth+midi: steps (28*32) + tail (playback 2 + sync 1 + dir 1 + automation 12*192); gap +900..935 NOT counted
        + (2 * (2 * 8 * (32 * 28 + 2 + 1 + 1 + 12 * 192)))
        + (4 * 8 * (2 + 1 + 1 + 8 * 192)); // drum tail/pattern (gap +132..167 NOT counted)
    let total = data.len();
    println!(
        "Parsed/carried bytes: {} / {} ({:.2}%) | typed: timing, synth+midi(steps+tail), drums(+tail), scale, fx; automation carried RAW (values not allowlist-checked); legacy: scenes+chains; ~2.7% still unmapped (per-pattern 36B gaps, track_info, octaves, header)",
        known, total, (known as f64) * 100.0 / (total.max(1) as f64)
    );

    // ---- typed header ----
    let t = &sess.timing;
    println!("Timing: tempo={} swing={} swing_sync_rate={} spare1={} spare2={}",
             t.tempo, t.swing, t.swing_sync_rate, t.spare1, t.spare2);
    println!("Scale: root={} type={}", sess.scale.root, sess.scale.scale_type);
    println!("FX: delay_preset={} reverb_preset={}", sess.fx.delay_preset, sess.fx.reverb_preset);

    // ---- scenes & chains (legacy) ----
    let scenes_offsets = ScenesOffsets { base: 0x40, scene_stride: 0x28, entry_stride: 4 };
    let _scenes = Scenes::from_bytes(&data, &scenes_offsets)?;
    let chain_offsets = ChainOffsets { scene_chain_base: 0x2C0, pattern_chain_base: 0x2C4, pattern_chain_stride: 4 };
    let scene_chain = SceneChain::from_bytes(&data, &chain_offsets)?;
    let _pattern_chains = PatternChains::from_bytes(&data, &chain_offsets)?;
    println!("Scenes: 16x8 parsed | SceneChain: {}..{} | PatternChains: 8 entries",
             scene_chain.start_scene, scene_chain.end_scene);

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

    fn load_drums(path: &str) -> DrumData {
        let data = read_file(path).expect("failed to read test ncs file");
        let offsets = default_drum_offsets();
        DrumData::from_bytes(&data, &offsets).expect("parse drums")
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

        let drums = DrumData::from_bytes(&data, &offsets).expect("parse drums");
        let steps = &drums.tracks[0].patterns[0].steps;
        assert_eq!(steps[0].velocity, 127);
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

        let drums = DrumData::from_bytes(&data, &offsets).expect("parse drums");
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

