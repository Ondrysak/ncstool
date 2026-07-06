//! `.circuittrackspack` support: a ZIP container holding
//!   - `projects/project_N.ncs` — sessions (parsed via the typed [`crate::session::Session`])
//!   - `samples/sample_N.wav`    — drum samples (indexed by drumChoice/defaultDrumChoices)
//!   - `patches/patch_N.syx`     — synth presets (indexed by synth_track_info.patch)
//!   - `index.json`              — manifest mapping every index above to a human name
//!
//! This module resolves the raw indices the session parser reads into the names
//! from `index.json`, so a whole pack can be browsed as human-readable sessions.

use std::io::{self, Read};

use serde::Deserialize;

use crate::session::Session;

#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(default)]
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Index {
    #[serde(default)]
    name: String,
    #[serde(default)]
    product: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    projects: Vec<Entry>,
    #[serde(default)]
    samples: Vec<Entry>,
    #[serde(default)]
    patches: Vec<Entry>,
}

/// An opened pack: the manifest plus its ZIP archive.
pub struct Pack {
    index: Index,
    zip: zip::ZipArchive<std::fs::File>,
}

impl Pack {
    pub fn open(path: &str) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("not a zip: {e}")))?;
        let index_bytes = read_entry(&mut zip, "index.json")?;
        let index: Index = serde_json::from_slice(&index_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad index.json: {e}")))?;
        Ok(Pack { index, zip })
    }

    /// Resolve a sample index to its name (from index.json), if present.
    fn sample_name(&self, i: usize) -> &str {
        self.index.samples.get(i).map(|e| e.name.as_str()).unwrap_or("<unknown>")
    }

    /// Resolve a patch index to its name (from index.json), if present.
    fn patch_name(&self, i: usize) -> &str {
        self.index.patches.get(i).map(|e| e.name.as_str()).unwrap_or("<unknown>")
    }

    /// Print a pack overview + a resolved summary of every project that is
    /// actually present in the archive (the manifest lists 64 slots but only
    /// the used ones exist as entries).
    pub fn summarize(&mut self) -> io::Result<()> {
        println!(
            "Pack: {:?} | product={} version={} | {} project slots, {} samples, {} patches",
            self.index.name, self.index.product, self.index.version,
            self.index.projects.len(), self.index.samples.len(), self.index.patches.len()
        );

        // Collect (name, url) for present projects first (avoids borrow issues).
        let present: Vec<(String, String)> = self
            .index
            .projects
            .iter()
            .filter_map(|p| {
                let name = if p.name.is_empty() { "<unnamed>".to_string() } else { p.name.clone() };
                self.zip.index_for_name(&p.url).map(|_| (name, p.url.clone()))
            })
            .collect();

        println!("\n{} project(s) present:", present.len());
        for (name, url) in &present {
            let bytes = read_entry(&mut self.zip, url)?;
            match Session::parse(&bytes) {
                Ok(sess) => self.print_project(name, url, &sess),
                Err(e) => println!("  {name} ({url}): parse error: {e}"),
            }
        }
        Ok(())
    }

    fn print_project(&self, name: &str, url: &str, sess: &Session) {
        println!("\n=== {name}  ({url}) ===");
        println!("  tempo {}  scale root {} type {}  fx delay {} reverb {}",
                 sess.timing.tempo, sess.scale.root, sess.scale.scale_type,
                 sess.fx.delay_preset, sess.fx.reverb_preset);
        // Drum tracks: default choice index -> sample name
        for (t, ch) in sess.default_drum_choices.iter().enumerate() {
            println!("  drum {}: choice {:>2} = \"{}\"", t + 1, ch, self.sample_name(*ch as usize));
        }
        // Synth tracks: patch index -> patch name
        for (t, info) in sess.synth_track_info.iter().enumerate() {
            println!("  synth {}: patch {:>3} = \"{}\"", t + 1, info.patch, self.patch_name(info.patch as usize));
        }
    }
}

/// Read a single archive entry fully into memory.
fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> io::Result<Vec<u8>> {
    let mut f = zip
        .by_name(name)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("missing {name}: {e}")))?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Entry point for the `pack` subcommand.
pub fn run_pack(path: &str) -> io::Result<()> {
    let mut pack = Pack::open(path)?;
    pack.summarize()
}
