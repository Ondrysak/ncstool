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

    let drums = DrumData::from_bytes(&data, &offsets)?;
    println!("{:#?}", drums);

    Ok(())
}
