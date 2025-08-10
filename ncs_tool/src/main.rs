use std::fs::File;
use std::io::{self, Read};

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
    let mut count_plane = |base: usize| {
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


fn main() -> io::Result<()> {
    let file_path = std::env::args().nth(1).expect("Usage: <program> <ncs file>");
    let data = read_file(&file_path)?;

    // Example offsets, adjust for your NCS layout
    let offsets = Offsets {
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
    };

    // Offsets from reverse engineering analysis
    let fx_offsets = FxOffsets {
        delay_preset: 0x00026D0E, // (&DAT_ram_00026d0e)[param1]
        reverb_preset: 0x00026D0F, // (&DAT_ram_00026d0f)[param1]
    };
    let timing_offsets = TimingOffsets { tempo: 0x34, swing: 0x35, swing_sync_rate: 0x36, spare1: 0x38, spare2: 0x3C };
    let scale_offsets = ScaleOffsets { root: 0x26D0C, scale_type: 0x26D0D };


    let timing = Timing::from_bytes(&data, &timing_offsets)?;
    let scale = ScaleSettings::from_bytes(&data, &scale_offsets)?;

    let fx = Fx::from_bytes(&data, &fx_offsets)?;

    let drums = DrumData::from_bytes(&data, &offsets)?;

    // Simple coverage metric
    let known = compute_known_bytes(&data, &offsets, &fx_offsets)
        + 3  // timing bytes: tempo, swing, swing_sync_rate
        + 8  // timing dwords: spare1, spare2
        + (16 * 8 * 4)  // scenes table bytes
        + 4              // scene chain: start,end,pad u16
        + (8 * 4);       // pattern chains: 8 entries x 4 bytes

    let total = data.len();

    println!(
        "Known bytes: {} / {} ({:.2}%) | fields: steps[velocity,probability,choice,mask], fx[delay,reverb], timing[tempo,swing,swing_sync_rate,spare1,spare2], scale[root,type], scenes+chains",
        known,
        total,
        (known as f64) * 100.0 / (total.max(1) as f64)
    );

    // ASCII/debug header
    println!("Timing: tempo={} swing={} swing_sync_rate={} spare1={} spare2={}", timing.tempo, timing.swing, timing.swing_sync_rate, timing.spare1, timing.spare2);
    println!("Scale: root={} type={}", scale.root, scale.scale_type);

    println!("FX: delay_preset={} reverb_preset={}", fx.delay_preset, fx.reverb_preset);

    // Scenes & chains
    let scenes_offsets = ScenesOffsets { base: 0x40, scene_stride: 0x28, entry_stride: 4 };
    let _scenes = Scenes::from_bytes(&data, &scenes_offsets)?;
    let chain_offsets = ChainOffsets { scene_chain_base: 0x2C0, pattern_chain_base: 0x2C4, pattern_chain_stride: 4 };
    let scene_chain = SceneChain::from_bytes(&data, &chain_offsets)?;
    let _pattern_chains = PatternChains::from_bytes(&data, &chain_offsets)?;
    println!("Scenes: 16x8 parsed | SceneChain: {}..{} | PatternChains: 8 entries",
             scene_chain.start_scene, scene_chain.end_scene);




    // Drums (ASCII)
    for t in 0..TRACKS {
        println!("\n=== DRUM TRACK {} ===", t);
        for p in 0..PATTERNS {
            let patt = &drums.tracks[t].patterns[p];
            let ascii = render_ascii(&patt.steps, true);
            let label = format!("P{:02}: ", p);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn load_drums(path: &str) -> DrumData {
        let data = read_file(path).expect("failed to read test ncs file");
        let offsets = Offsets {
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
        };
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
}

