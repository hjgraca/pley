//! How fast names are read: every reference name, many times over.
//! `cargo run --release -p pley-releases --example speed`

use std::time::Instant;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
        names.extend(doc["cases"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str().map(str::to_owned)));
    }
    let started = Instant::now();
    pley_releases::parse(&names[0]);
    println!("first name (patterns compiled): {:?}", started.elapsed());
    let rounds = 20;
    let started = Instant::now();
    for _ in 0..rounds {
        for name in &names {
            std::hint::black_box(pley_releases::parse(name));
        }
    }
    let per = started.elapsed() / (rounds * names.len() as u32);
    println!("{} names x {rounds}: {per:?} a name, {:.0} names a second", names.len(), 1.0 / per.as_secs_f64());
}
