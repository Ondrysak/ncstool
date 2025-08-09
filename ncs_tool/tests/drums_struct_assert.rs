use std::fs;

#[derive(Debug)]
struct HitSpec {
    // Expect velocity constraints for a step
    min: u8,
    max: u8,
}

fn load_bytes(rel: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read(path).expect("failed to read test ncs file")
}

// These offsets mirror the ones used by the binary for now
const VELOCITY_OFF: usize = 0x0CD74;
const PROB_OFF: usize = 0x0CD94;
const CHOICE_OFF: usize = 0x0CDB4;
const MASK_OFF: usize = 0x0CDD4;
const PITCH_OFF: usize = 0x0CDF4;
const DECAY_OFF: usize = 0x0CE14;
const DIST_OFF: usize = 0x0CE34;
const EQ_OFF: usize = 0x0CE54;
const TRACK_STRIDE: usize = 0x3540;
const PATTERN_STRIDE: usize = 0x06A8;

fn step_idx(track: usize, pat: usize, step: usize) -> usize {
    track * TRACK_STRIDE + pat * PATTERN_STRIDE + step
}

fn load_deep_track2_p2() -> Vec<(u8,u8)> {
    let data = load_bytes("../test_data/Deep.ncs");
    (0..32).map(|s| {
        let i = step_idx(2, 2, s);
        (data[VELOCITY_OFF + i], data[PROB_OFF + i])
    }).collect()
}

fn load_funk_track2_p2() -> Vec<(u8,u8)> {
    let data = load_bytes("../test_data/Funk.ncs");
    (0..32).map(|s| {
        let i = step_idx(2, 2, s);
        (data[VELOCITY_OFF + i], data[PROB_OFF + i])
    }).collect()
}

#[test]
fn deep_track2_p2_structure() {
    let steps = load_deep_track2_p2();
    // First bar: strong, weak, rest, weak, strong, weak, rest, weak
    let weak = |v: u8| v > 0 && v <= 31;
    assert!(steps[0].0 >= 96);
    assert!(weak(steps[1].0));
    assert_eq!(steps[2].0, 0);
    assert!(weak(steps[3].0));
    assert!(steps[4].0 >= 96);
    assert!(weak(steps[5].0));
    assert_eq!(steps[6].0, 0);
    assert!(weak(steps[7].0));
    // Probability digit is 7 for played hits in our sample
    for &i in &[0usize,1,3,4,5,7] {
        assert_eq!(steps[i].1, 7, "Deep prob mismatch at step {}", i);
    }
}

#[test]
fn funk_track2_p2_structure() {
    let steps = load_funk_track2_p2();
    // First bar: strong, weak, weak, weak, strong, weak, weak, weak
    let weak = |v: u8| v > 0 && v <= 31;
    assert!(steps[0].0 >= 96);
    for &i in &[1usize,2,3,5,6,7] { assert!(weak(steps[i].0), "Funk weak expected at step {}", i); }
    assert!(steps[4].0 >= 96);
    // Bars 3 and 4: rests
    for i in 16..32 { assert_eq!(steps[i].0, 0, "Funk rest expected at step {}", i); }
}

