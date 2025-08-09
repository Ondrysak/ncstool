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

    // FX offsets; validate_fx_presets() shows 1 byte each for delay (0..15) and reverb (0..7)
    // These offsets are placeholders; adjust once confirmed from the buffer layout.
    let fx_offsets = FxOffsets {
        delay_preset: 0x00026D0E, // (&DAT_ram_00026d0e)[param1]
        reverb_preset: 0x00026D0F, // (&DAT_ram_00026d0f)[param1]
    };


    let fx = Fx::from_bytes(&data, &fx_offsets)?;
    println!("FX: delay_preset={} reverb_preset={}", fx.delay_preset, fx.reverb_preset);

    let drums = DrumData::from_bytes(&data, &offsets)?;
    println!("{:#?}", drums);

    // ASCII output similar to the Python CLI: per track and pattern
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

