//! Sonarr's and Radarr's own parser test names (`tests/reference/*.json`,
//! taken from their `ParserTests` fixtures), read by this crate: how many
//! come out as they expect. Each kind has a floor the share may not fall
//! below; `cargo test -p pley-releases --test reference -- --nocapture`
//! prints the shares and the names read wrong.

use std::path::Path;

use pley_releases::{Modifier, Release, Source, parse};
use serde_json::Value;

/// Titles as a name is matched to a show or a movie: letters and digits,
/// lowercased, `&` read as `and`.
fn clean(title: &str) -> String {
    title.to_lowercase().replace('&', "and").chars().filter(|c| c.is_alphanumeric()).collect()
}

fn ints(value: &Value) -> Vec<u32> {
    value.as_array().map(|a| a.iter().filter_map(|v| v.as_u64()).map(|v| v as u32).collect()).unwrap_or_default()
}

fn source(name: &str) -> Source {
    match name {
        "cam" => Source::Cam,
        "telesync" => Source::Telesync,
        "telecine" => Source::Telecine,
        "workprint" => Source::Workprint,
        "dvd" => Source::Dvd,
        "tv" => Source::Tv,
        "webrip" => Source::Webrip,
        "webdl" => Source::Webdl,
        "bluray" => Source::Bluray,
        _ => Source::Unknown,
    }
}

fn modifier(name: &str) -> Option<Modifier> {
    match name {
        "screener" => Some(Modifier::Screener),
        "regional" => Some(Modifier::Regional),
        "rawhd" => Some(Modifier::Rawhd),
        "brdisk" => Some(Modifier::Brdisk),
        "remux" => Some(Modifier::Remux),
        _ => None,
    }
}

/// Whether the release is read as the case expects, and what was read.
type Check = fn(&Value, &Release) -> bool;

/// Sonarr keeps a show's year in its title (`Show Title 2010`), and an
/// anime's season when a running number follows (`Show S3 - 12` is "Show
/// S3", episode 12); this crate reads them as the year and the season, and
/// keeps the running number beside it.
fn title(case: &Value, r: &Release) -> bool {
    let want = clean(case["title"].as_str().unwrap_or_default());
    let mine = clean(&r.title);
    let with_year = r.year.map(|y| format!("{mine}{y}"));
    let with_season = match r.seasons[..] {
        [season] if !r.absolute.is_empty() => {
            [format!("{mine}s{season}"), format!("{mine}s{season:02}")].contains(&want)
        }
        _ => false,
    };
    mine == want || with_year.is_some_and(|t| t == want) || with_season
}

fn episodes(case: &Value, r: &Release) -> bool {
    title(case, r) && r.seasons == [case["season"].as_u64().unwrap() as u32] && r.episodes == ints(&case["episodes"])
}

fn seasons(case: &Value, r: &Release) -> bool {
    title(case, r) && r.seasons == [case["season"].as_u64().unwrap() as u32] && r.episodes.is_empty()
}

fn daily(case: &Value, r: &Release) -> bool {
    title(case, r) && r.date.as_deref() == case["date"].as_str()
}

fn absolute(case: &Value, r: &Release) -> bool {
    let number = case["absolute"].as_u64().unwrap() as u32;
    // Some names give a season and episode beside the running number.
    let with_season = case.get("season").is_some_and(|s| r.seasons == [s.as_u64().unwrap() as u32]);
    title(case, r) && (r.absolute == [number] || with_season)
}

fn group(case: &Value, r: &Release) -> bool {
    r.group.as_deref() == case["group"].as_str()
}

fn year(case: &Value, r: &Release) -> bool {
    r.year.map(u64::from) == case["year"].as_u64()
}

fn quality(case: &Value, r: &Release) -> bool {
    let q = &case["quality"];
    let resolution = q["resolution"].as_u64().map(|v| v as u32);
    r.quality.source == source(q["source"].as_str().unwrap())
        && (resolution.is_none() || r.quality.resolution == resolution)
        && r.quality.modifier == q["modifier"].as_str().and_then(modifier)
        && (r.revision.version > 1) == case["proper"].as_bool().unwrap_or(false)
}

/// Each reference file, what is checked, and the share it must reach: what
/// it reached when the floors were set, so a change that reads names worse fails.
const KINDS: &[(&str, Check, f64)] = &[
    ("sonarr_episodes", episodes, 0.9),
    ("sonarr_seasons", seasons, 0.86),
    ("sonarr_daily", daily, 0.93),
    ("sonarr_absolute", absolute, 0.88),
    ("sonarr_groups", group, 1.0),
    ("sonarr_quality", quality, 0.97),
    ("radarr_titles", title, 0.86),
    ("radarr_years", year, 0.8),
    ("radarr_groups", group, 1.0),
    ("radarr_quality", quality, 0.98),
];

#[test]
fn sonarr_and_radarr_names_read_as_they_expect() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
    let (mut all, mut all_ok, mut below) = (0, 0, Vec::new());
    for &(kind, check, floor) in KINDS {
        let text = std::fs::read_to_string(dir.join(format!("{kind}.json"))).unwrap();
        let doc: Value = serde_json::from_str(&text).unwrap();
        let cases = doc["cases"].as_array().unwrap();
        let mut wrong = Vec::new();
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let read = parse(name);
            if !check(case, &read) {
                wrong.push((name, read));
            }
        }
        let ok = cases.len() - wrong.len();
        let share = ok as f64 / cases.len() as f64;
        println!("{kind:<18} {ok:>4}/{:<4} {:>5.1}%", cases.len(), share * 100.0);
        for (name, read) in
            wrong.iter().take(std::env::var("REFERENCE_WRONG").ok().and_then(|n| n.parse().ok()).unwrap_or(8))
        {
            println!(
                "    {name}\n      title {:?} year {:?} seasons {:?} episodes {:?} absolute {:?} date {:?} \
                 quality {:?} revision {:?} group {:?}",
                read.title,
                read.year,
                read.seasons,
                read.episodes,
                read.absolute,
                read.date,
                read.quality,
                read.revision,
                read.group
            );
        }
        all += cases.len();
        all_ok += ok;
        if share < floor {
            below.push(format!("{kind}: {:.1}% < {:.1}%", share * 100.0, floor * 100.0));
        }
    }
    println!("{:<18} {all_ok:>4}/{all:<4} {:>5.1}%", "all", all_ok as f64 / all as f64 * 100.0);
    assert!(below.is_empty(), "below the floor: {below:?}");
}
