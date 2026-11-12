//! Release names (`Show.S01E02.1080p.WEB-DL.DDP5.1.H.264-NTb`) read into what
//! they hold: the title, the year, seasons and episodes, an air date or an
//! anime's running number, the quality and the release group.
//!
//! Titles, years, seasons, episodes and dates are read by
//! `parse-torrent-title` (PTT), whose rules Torrentio and Comet run on
//! millions of names. The quality is read as Radarr reads it ([`quality`])
//! and the release group as Sonarr does ([`group`]): theirs are the rules
//! the indexers' names were written against. A quality is a source, a
//! resolution and a modifier, as Radarr has them; `tests/reference`
//! measures the whole against Sonarr's and Radarr's own test names.

mod group;
mod numbers;
mod quality;

pub use group::release_group;
pub use quality::{Revision, quality, revision};

use serde::Serialize;

/// What a release name says.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Release {
    pub title: String,
    pub year: Option<u16>,
    /// Seasons the release holds: one for an episode or a season pack,
    /// several for a multi-season pack, none for a movie or an anime
    /// numbered from its first episode.
    pub seasons: Vec<u32>,
    /// Episodes within its season.
    pub episodes: Vec<u32>,
    /// Anime's running numbers (`Show - 112`). A name that reads two ways
    /// (`Show - 101`) has both this and a season and episode; the show's
    /// kind picks one.
    pub absolute: Vec<u32>,
    /// The air date of a daily show's episode, `YYYY-MM-DD`.
    pub date: Option<String>,
    pub quality: Quality,
    /// A PROPER, a REPACK or a `v2`: the same release made again, which
    /// replaces it.
    pub revision: Revision,
    pub group: Option<String>,
    pub languages: Vec<String>,
    /// The whole series or collection (`COMPLETE`).
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Quality {
    pub source: Source,
    /// Lines of the picture: 480, 576, 720, 1080, 2160.
    pub resolution: Option<u32>,
    pub modifier: Option<Modifier>,
}

/// Where the picture comes from, worst to best.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Unknown,
    Cam,
    Telesync,
    Telecine,
    Workprint,
    Dvd,
    Tv,
    Webrip,
    Webdl,
    Bluray,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    /// A screener sent ahead of release.
    Screener,
    /// A region's early DVD (R5).
    Regional,
    /// A broadcast's raw stream (MPEG-2 at 1080i).
    Rawhd,
    /// A whole disc, as it was.
    Brdisk,
    /// The disc's video and audio, untouched, in a new container.
    Remux,
}

/// Reads a release name.
pub fn parse(name: &str) -> Release {
    let p = parse_torrent_title::parse(name);
    let ints = |values: Option<&[i64]>| -> Vec<u32> {
        values.unwrap_or_default().iter().filter_map(|&v| u32::try_from(v).ok()).collect()
    };
    let flag = |field: &str| p.get(field).is_some_and(|v| v.is_truthy());
    let seasons = ints(p.seasons());
    let episodes = ints(p.episodes());
    let (episodes, absolute) = if seasons.is_empty() { (Vec::new(), episodes) } else { (episodes, Vec::new()) };
    let mut release = Release {
        title: p.title().to_owned(),
        year: p.year().and_then(|v| v.as_int()).and_then(|y| u16::try_from(y).ok()),
        seasons,
        episodes,
        absolute,
        date: p.date().map(str::to_owned),
        quality: quality(name),
        revision: revision(name),
        group: release_group(name),
        languages: p.languages().unwrap_or_default().to_vec(),
        complete: flag("complete"),
    };
    numbers::complete(name, &mut release);
    release
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_episode() {
        let r = parse("Show.Title.S01E02.1080p.WEB-DL.DDP5.1.H.264-NTb");
        assert_eq!((r.title.as_str(), &r.seasons[..], &r.episodes[..]), ("Show Title", &[1][..], &[2][..]));
        assert_eq!(r.quality, Quality { source: Source::Webdl, resolution: Some(1080), modifier: None });
        assert_eq!(r.group.as_deref(), Some("NTb"));
    }

    #[test]
    fn a_movie_remux() {
        let r = parse("Movie.Title.2019.2160p.UHD.BluRay.REMUX.HDR.HEVC.Atmos-GROUP");
        assert_eq!((r.title.as_str(), r.year), ("Movie Title", Some(2019)));
        assert_eq!(
            r.quality,
            Quality { source: Source::Bluray, resolution: Some(2160), modifier: Some(Modifier::Remux) }
        );
    }

    #[test]
    fn an_anime_running_number() {
        let r = parse("[SubsPlease] Some Anime Show - 112 (1080p) [ABCD1234].mkv");
        assert!(r.seasons.is_empty());
        assert_eq!(r.absolute, [112]);
    }
}
