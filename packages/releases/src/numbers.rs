//! Episode numbering PTT leaves in the title or reads otherwise than Sonarr,
//! which the indexers' names follow: `Show.103.HDTV` is season 1, episode 3;
//! `[Group] Show 214` an anime's 214th episode; `Part 2` and `5of9` a
//! miniseries' episodes; `S41 E10478`, `2009x09`; and air dates written
//! `140722` or year, day, month. Each runs only where PTT found nothing of
//! its kind.
//!
//! Some names read two ways, and only the show tells which: `Show - 101` is
//! an anime's 101st episode or season 1, episode 1; `[Group] Show S3 - 12`
//! is season 3, episode 12 or an anime's 12th. Both readings are kept (the
//! running number in `absolute`, beside the season and episode), and the
//! show's kind picks one.

use std::sync::LazyLock;

use parse_torrent_title::Regex;

use crate::Release;

fn re(source: &str) -> Regex {
    Regex::new(source, "i").expect("a numbering pattern")
}

fn group(m: &parse_torrent_title::Match, text: &str, name: &str) -> Option<u32> {
    m.named_group(name).and_then(|r| text[r].parse().ok())
}

/// A name that opens with its episode code or air date has no show title.
static LEADING_CODE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:S\d+E\d+|\d{1,4}x\d{1,3}|(?:19|20)\d{2}[-._ ]?\d{2}[-._ ]?\d{2})(?:\b|_)"));

/// `[Group]` opening the name: an anime fansub's release.
static ANIME: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*\[[^\]]+\]"));

/// The title's last word a number: `Show 214`, `Show-01`.
static TRAILING_NUMBER: LazyLock<Regex> = LazyLock::new(|| re(r"^(?<title>.+?)[ _-]+(?<number>\d{1,4})$"));

/// `Part 2`, `Part.Two`, `Part01` at the title's end.
static PART: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(?<title>.+?)[ ._-]+Part[ ._-]*(?<number>\d{1,2}|One|Two|Three|Four|Five|Six|Seven|Eight|Nine|Ten)$")
});

/// `5of9`, `Part 5 of 9`: a miniseries' fifth episode.
static OF: LazyLock<Regex> = LazyLock::new(|| re(r"[ ._-](?<number>\d{1,2})[ ._-]?of[ ._-]?\d{1,2}[ ._-]"));

/// `E1135`: an episode number with no season, which stays a running number.
static BARE_EPISODE: LazyLock<Regex> = LazyLock::new(|| re(r"(?:^|[ ._-])E(?:P)?\d{3,4}(?:[ ._-]|$)"));

/// `S3 - 12`, `S2 - Episode 4`: a season, then an anime's running number.
static SEASON_DASH_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| re(r"\bS\d{1,2}\s*-\s*(?:EP?|Episode\s*)?\s*(?<number>\d{1,4})\b"));

/// The first episode code of a multi-episode name: `S03E01`, `2x04`,
/// `S42 Ep10718`.
static FIRST_EPISODE: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?:\bS(?<s>\d{1,4})[ ._]?E[Pp]?|(?:^|[ ._\[(-])(?<s2>\d{1,4})x|\bS(?<s3>\d{1,4})[ ._]+Ep?)(?<e>\d{1,5})(?!\d)")
});

/// What follows an episode code to make it several: `-06`, `-E03`,
/// `.S03E02`, `x05`, `-x02`, `_02`, ` - 2x10`.
static NEXT_EPISODE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?<sep>[ ._]*-[ ._]*|[ ._]+|_|)(?:(?:S(?<s>\d{1,4})[ ._]?)?E[Pp]?|(?<s2>\d{1,4})x|(?<x>x)|(?<bare>))(?<e>\d{1,5})(?!\d)",
    )
});

/// `103.104`, `10708`: episodes of one season run together.
static COMPACT_EPISODES: LazyLock<Regex> =
    LazyLock::new(|| re(r"[ ._](?:(?<a>\d{3,4})[ ._](?<b>\d{3,4})|(?<s>\d)(?<c>\d{2})(?<d>\d{2}))(?=[ ._-]|$)"));

/// Spanish releases' `[Cap.408]` (season 4, episode 8), `[Cap.111_120]`.
static CAPITULO: LazyLock<Regex> =
    LazyLock::new(|| re(r"\bCap\.(?<s>\d{1,2})(?<e>\d{2})(?:_(?<s2>\d{1,2})(?<e2>\d{2}))?\b"));

/// `S41 E10478`: the episode apart from its season.
static SPACED_EPISODE: LazyLock<Regex> = LazyLock::new(|| re(r"\bS(?<season>\d{1,4})[ ._-]+E(?<episode>\d{1,5})\b"));

/// `2009x09`, `S2009E09`: a year as the season.
static YEAR_SEASON: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?:^|[ ._-])S?(?<season>(?:19|20)\d{2})[xE](?<episode>\d{2,3})\b"));

/// `140722` between separators: year, month and day of an air date.
static SIX_DIGIT_DATE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?<=[_. -])(?<year>\d{2})(?<month>[01]\d)(?<day>[0-3]\d)(?=[_. -]|$)"));

/// `2012.16.02`: year, day, month (the day above 12).
static YEAR_DAY_MONTH: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?:^|[ ._-])(?<year>(?:19|20)\d{2})[ ._-](?<day>1[3-9]|2\d|3[01])[ ._-](?<month>0?[1-9]|1[0-2])(?:[ ._-]|$)")
});

/// `1954-01-23`: an ISO date, which no other reading beats.
static ISO_DATE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?:^|[ ._(\[])(?<year>(?:19|20)\d{2})-(?<month>[01]\d)-(?<day>[0-3]\d)(?:$|[ ._)\]])"));

const NUMBER_WORDS: &[&str] = &["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];

fn number_word(word: &str) -> Option<u32> {
    word.parse().ok().or_else(|| NUMBER_WORDS.iter().position(|w| w.eq_ignore_ascii_case(word)).map(|i| i as u32 + 1))
}

fn is_year(n: u32) -> bool {
    (1900..=2099).contains(&n)
}

fn date(year: u32, month: u32, day: u32) -> Option<String> {
    ((1..=12).contains(&month) && (1..=31).contains(&day)).then(|| format!("{year:04}-{month:02}-{day:02}"))
}

/// Every episode of a multi-episode name, from its first code on: a range
/// where a hyphen joins two (`S03E01-06`), the episodes one by one
/// otherwise (`S03E01.S03E02`, `8x01_02`).
fn multi_episode(name: &str) -> Option<(u32, Vec<u32>)> {
    let first = FIRST_EPISODE.find(name)?;
    let season = ["s", "s2", "s3"].iter().find_map(|g| group(&first, name, g))?;
    let mut episodes = vec![group(&first, name, "e")?];
    let mut at = first.end();
    while let Some(m) = NEXT_EPISODE.find(&name[at..]) {
        let rest = &name[at..];
        let e = group(&m, rest, "e")?;
        let sep = m.named_group("sep").map(|r| &rest[r]).unwrap_or_default();
        let other_season = ["s", "s2"].iter().find_map(|g| group(&m, rest, g)).is_some_and(|s| s != season);
        // A bare number goes on only when joined to the code (`S03E01-06`,
        // `8x01_02`), not after ` - ` (`S02E21 - 94 Meetings` is a title)
        // nor before letters (`-1080p`); an `x` only after a hyphen or an `_`
        // (`.x264` is a codec).
        let after = rest[m.end()..].chars().next();
        let bare = m.named_group("bare").is_some()
            && !((sep == "-" || sep == "_") && !after.is_some_and(|c| c.is_ascii_alphanumeric()));
        let x = m.named_group("x").is_some() && !(sep.contains('-') || sep == "_" || sep.is_empty());
        if other_season || bare || x || m.range().is_empty() {
            break;
        }
        let last = *episodes.last()?;
        if e <= last {
            break;
        }
        if sep.contains('-') && e - last <= 50 {
            episodes.extend(last + 1..=e);
        } else {
            episodes.push(e);
        }
        at += m.end();
    }
    Some((season, episodes))
}

/// Fills in what PTT left out, from the name it read.
pub(crate) fn complete(name: &str, r: &mut Release) {
    if let Some(m) = CAPITULO.find(name)
        && let (Some(s), Some(e)) = (group(&m, name, "s"), group(&m, name, "e"))
    {
        r.seasons = vec![s];
        r.episodes = match group(&m, name, "e2") {
            Some(last) if last >= e && group(&m, name, "s2") == Some(s) => (e..=last).collect(),
            _ => vec![e],
        };
        r.absolute.clear();
    }
    if r.seasons.len() == 1
        && r.episodes.len() <= 1
        && let Some((season, episodes)) = multi_episode(name)
        && season == r.seasons[0]
        && episodes.len() > r.episodes.len()
    {
        r.episodes = episodes;
    }
    let anime = ANIME.is_match(name);
    let none = |r: &Release| r.seasons.is_empty() && r.absolute.is_empty() && r.date.is_none();

    if let Some(m) = ISO_DATE.find(name)
        && let (Some(y), Some(mo), Some(d)) =
            (group(&m, name, "year"), group(&m, name, "month"), group(&m, name, "day"))
        && let Some(iso) = date(y, mo, d)
    {
        r.date = Some(iso);
    }

    if r.date.is_none() && r.seasons.is_empty() {
        if let Some(m) = YEAR_DAY_MONTH.find(name) {
            let (y, d, mo) = (group(&m, name, "year"), group(&m, name, "day"), group(&m, name, "month"));
            if let (Some(y), Some(d), Some(mo)) = (y, d, mo) {
                r.date = date(y, mo, d);
                if r.year == Some(y as u16) {
                    r.year = None;
                }
            }
        } else if let Some(m) = SIX_DIGIT_DATE.find(name) {
            let (y, mo, d) = (group(&m, name, "year"), group(&m, name, "month"), group(&m, name, "day"));
            if let (Some(y), Some(mo), Some(d)) = (y, mo, d) {
                r.date = date(2000 + y, mo, d);
                if r.date.is_some() {
                    let digits = &name[m.range()];
                    r.title = r.title.trim_end_matches(digits).trim_end().to_owned();
                }
            }
        }
    }

    if !r.seasons.is_empty()
        && r.episodes.is_empty()
        && let Some(m) = SPACED_EPISODE.find(name)
        && let Some(e) = group(&m, name, "episode")
    {
        r.episodes = vec![e];
    }

    if r.seasons.is_empty()
        && let Some(m) = YEAR_SEASON.find(name)
        && let (Some(s), Some(e)) = (group(&m, name, "season"), group(&m, name, "episode"))
    {
        r.seasons = vec![s];
        r.episodes = vec![e];
        r.absolute.clear();
        if r.year == Some(s as u16) {
            r.year = None;
        }
    }

    if !ANIME.is_match(name)
        && r.date.is_none()
        && r.episodes.len() <= 1
        && let Some(m) = COMPACT_EPISODES.find(name)
    {
        let (a, b) = (group(&m, name, "a"), group(&m, name, "b"));
        let pair = match (a, b) {
            (Some(a), Some(b)) if a / 100 == b / 100 && b > a && !is_year(a) && !is_year(b) => {
                Some((a / 100, vec![a % 100, b % 100]))
            }
            _ => match (group(&m, name, "s"), group(&m, name, "c"), group(&m, name, "d")) {
                (Some(s), Some(c), Some(d)) if s > 0 && d == c + 1 => Some((s, vec![c, d])),
                _ => None,
            },
        };
        if let Some((season, episodes)) = pair {
            let digits = name[m.range()].trim_matches(|c: char| !c.is_ascii_digit());
            let first = digits.split(|c: char| !c.is_ascii_digit()).next().unwrap_or(digits);
            if let Some(i) = r.title.find(first) {
                r.title = r.title[..i].trim_end().to_owned();
            }
            r.seasons = vec![season];
            r.episodes = episodes;
            r.absolute.clear();
        }
    }

    // Numbers left at the title's end: an anime's running number, else a
    // season and episode run together (`103`).
    if none(r) {
        if let Some(m) = PART.find(&r.title.clone()) {
            let title = r.title.clone();
            if let Some(n) = m.named_group("number").and_then(|g| number_word(&title[g])) {
                r.title = m.named_group("title").map(|g| title[g].to_owned()).unwrap_or_default();
                r.seasons = vec![1];
                r.episodes = vec![n];
            }
        } else if let Some(m) = TRAILING_NUMBER.find(&r.title.clone()) {
            let title = r.title.clone();
            if let Some(n) = group(&m, &title, "number") {
                let rest = m.named_group("title").map(|g| title[g].to_owned()).unwrap_or_default();
                if anime {
                    r.title = rest;
                    r.absolute = vec![n];
                } else if (100..10_000).contains(&n) && !is_year(n) {
                    r.title = rest;
                    r.seasons = vec![n / 100];
                    r.episodes = vec![n % 100];
                    r.absolute = vec![n];
                }
            }
        }
    }

    // PTT reads `Show.2009.416` and `Show - 101` as running numbers; outside
    // anime they are a season and episode run together.
    if !anime
        && r.seasons.is_empty()
        && r.date.is_none()
        && let [n] = r.absolute[..]
    {
        if (100..10_000).contains(&n) && !is_year(n) && !BARE_EPISODE.is_match(name) {
            r.seasons = vec![n / 100];
            r.episodes = vec![n % 100];
        } else if let Some(m) = OF.find(name)
            && let Some(e) = group(&m, name, "number")
        {
            r.seasons = vec![1];
            r.episodes = vec![e];
            r.absolute.clear();
        }
    }

    if r.absolute.is_empty()
        && r.seasons.len() == 1
        && let Some(m) = SEASON_DASH_NUMBER.find(name)
        && let Some(n) = group(&m, name, "number")
    {
        r.absolute = vec![n];
    }

    if LEADING_CODE.is_match(r.title.trim()) || LEADING_CODE.is_match(name.trim()) {
        r.title.clear();
    }
}
