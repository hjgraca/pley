//! A release's quality, read as Radarr reads it (`QualityParser`): the source
//! word that comes last, the resolution, a remux or a whole disc, and what a
//! name without them most likely is (an episode with no source word was on
//! TV; an anime release with `[WEB]` is a WEB-DL). Its patterns run on PTT's
//! ECMAScript engine, which takes .NET's look-arounds as they are.

use std::sync::LazyLock;

use parse_torrent_title::Regex;

use crate::{Modifier, Quality, Source};

fn re(source: &str, flags: &str) -> Regex {
    Regex::new(source, flags).expect("a quality pattern")
}

/// The source words. The last in a name wins (`…HDTV.WEB-DL…` is a WEB-DL).
static SOURCE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        concat!(
            r"\b(?:",
            r"(?<bluray>M?Blu[-_. ]?Ray|HD[-_. ]?DVD|BD(?!$)|UHD2?BD|BDISO|BDMux|BD25|BD50|BR[-_. ]?DISK)|",
            r"(?<webdl>WEB[-_. ]?DL(?:mux)?|AmazonHD|AmazonSD|iTunesHD|MaxdomeHD|NetflixU?HD|WebHD|HBOMaxHD|DisneyHD",
            r"|[. ]WEB[. ](?:[xh][ .]?26[45]|AVC|HEVC|DDP?5[. ]1)|[. ]WEB$|(?:\d{3,4}0p)[-. ](?:Hybrid[-_. ]?)?WEB[-. ]",
            r"|[-. ]WEB[-. ]\d{3,4}0p|\b\s\/\sWEB\s\/\s\b|(?:AMZN|NF|DP)[. -]WEB[. -](?!Rip))|",
            r"(?<webrip>WebRip|Web-Rip|WEBMux)|",
            r"(?<hdtv>HDTV)|",
            r"(?<bdrip>BDRip|BDLight|HD[-_. ]?DVDRip|UHDBDRip)|",
            r"(?<brrip>BRRip)|",
            r"(?<dvdr>\d?x?M?DVD-?[R59])|",
            r"(?<dvd>DVD(?!-R)|DVDRip|xvidvd)|",
            r"(?<dsr>WS[-_. ]DSR|DSR)|",
            r"(?<regional>R[0-9]{1}|REGIONAL)|",
            r"(?<scr>SCR|SCREENER|DVDSCR|DVDSCREENER)|",
            r"(?<ts>TS[-_. ]|TELESYNCH?|HD-TS|HDTS|PDVD|TSRip|HDTSRip)|",
            r"(?<tc>TC|TELECINE|HD-TC|HDTC)|",
            r"(?<cam>CAMRIP|(?:NEW)?CAM|HD-?CAM(?:Rip)?|HQCAM)|",
            r"(?<wp>WORKPRINT|WP)|",
            r"(?<pdtv>PDTV)|",
            r"(?<sdtv>SDTV)|",
            r"(?<tvrip>TVRip)",
            r")(?:\b|$|[ .])",
        ),
        "gi",
    )
});

static RAW_HD: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?<rawhd>RawHD|Raw[-_. ]HD)\b", "i"));
static MPEG2: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?<mpeg2>MPEG[-_. ]?2)\b", ""));

/// A whole Blu-ray disc (`COMPLETE.BLURAY`, `BD50`, an AVC stream on a
/// Blu-ray without an encode's words).
static BR_DISK: LazyLock<Regex> = LazyLock::new(|| {
    re(
        concat!(
            r"^(?!.*\b((?<!HD[._ -]|HD)DVD|BDRip|720p|MKV|XviD|WMV|d3g|(BD)?REMUX|^(?=.*1080p)(?=.*HEVC)|[xh][-_. ]?26[45]",
            r"|German.*[DM]L|((?<=\d{4}).*German.*([DM]L)?)(?=.*\b(AVC|HEVC|VC[-_. ]?1|MVC|MPEG[-_. ]?2)\b))\b)",
            r"(((?=.*\b(Blu[-_. ]?ray|BD|HD[-_. ]?DVD)\b)(?=.*\b(AVC|HEVC|VC[-_. ]?1|MVC|MPEG[-_. ]?2|BDMV|ISO)\b))",
            r"|^((?=.*\b(((?=.*\b((.*_)?COMPLETE.*|Dis[ck])\b)(?=.*(Blu[-_. ]?ray|HD[-_. ]?DVD)))|3D[-_. ]?BD|BR[-_. ]?DISK",
            r"|Full[-_. ]?Blu[-_. ]?ray|^((?=.*((BD|UHD)[-_. ]?(25|50|66|100|ISO)))))))).*",
        ),
        "i",
    )
});

static PROPER: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?<proper>proper)\b", "i"));
static REPACK: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?<repack>repack\d?|rerip\d?)\b", "i"));
static VERSION: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\d[-._ ]?v(?<version>\d)[-._ ]|\[v(?<version>\d)\]|repack(?<version>\d)|rerip(?<version>\d)", "i")
});

static RESOLUTION: LazyLock<Regex> = LazyLock::new(|| {
    re(
        concat!(
            r"\b(?:(?<R360p>360p)|(?<R480p>480p|480i|640x480|848x480)|(?<R540p>540p)|(?<R576p>576p)",
            r"|(?<R720p>720p|1280x720|960p)|(?<R1080p>1080p|1920x1080|1440p|FHD|1080i|4kto1080p)",
            r"|(?<R2160p>2160p|3840x2160|4k[-_. ](?:UHD|HEVC|BD|H\.?265)|(?:UHD|HEVC|BD|H\.?265)[-_. ]4k))\b",
        ),
        "i",
    )
});

/// No resolution word, but `UHD` or `[4K]`.
static ALTERNATIVE_RESOLUTION: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?<R2160p>UHD)\b|(?<R2160p>\[4K\])", "i"));

static CODEC: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(?:(?<x264>x264)|(?<h264>h264)|(?<xvidhd>XvidHD)|(?<xvid>X-?vid)|(?<divx>divx))\b", "i"));

static OTHER_SOURCE: LazyLock<Regex> = LazyLock::new(|| re(r"(?<hdtv>HD[-_. ]TV)|(?<sdtv>SD[-_. ]TV)", "i"));
static ANIME_BLURAY: LazyLock<Regex> = LazyLock::new(|| re(r"bd(?:720|1080|2160)|(?<=[-_. (\[])bd(?=[-_. )\]])", "i"));
static ANIME_WEBDL: LazyLock<Regex> = LazyLock::new(|| re(r"\[WEB\]|[\[\(]WEB[ .]", "i"));
static HIGH_DEF_PDTV: LazyLock<Regex> = LazyLock::new(|| re(r"hr[-_. ]ws", "i"));
static REMUX: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:[_. \[]|\d{4}p-|\bHybrid-)(?<remux>(?:(BD|UHD)[-_. ]?)?Remux)\b|(?<remux>(?:(BD|UHD)[-_. ]?)?Remux[_. ]\d{4}p)",
        "i",
    )
});
static GERMAN_REMUX: LazyLock<Regex> = LazyLock::new(|| {
    re(r"((?<=\d{4}).*German.*([DM]L)?)(?=.*\b(AVC|HEVC|VC[_. -]?1|MVC|MPEG[_. -]?2))(?=.*Blu-?ray)", "i")
});

/// What a release says about how it was made again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Revision {
    pub proper: bool,
    pub repack: bool,
    /// `v2` and the like, counted from 1; a PROPER or REPACK is one more.
    pub version: u32,
}

pub fn revision(name: &str) -> Revision {
    let name = name.replace('_', " ");
    let version = VERSION.find(&name).and_then(|m| m.named_group("version")).and_then(|r| name[r].parse::<u32>().ok());
    let proper = PROPER.is_match(&name);
    let repack = REPACK.is_match(&name);
    let version = if proper || repack { version.map_or(2, |v| v + 1) } else { version.unwrap_or(1) };
    Revision { proper, repack, version }
}

const fn q(source: Source, resolution: Option<u32>, modifier: Option<Modifier>) -> Quality {
    Quality { source, resolution, modifier }
}

const UNKNOWN: Quality = q(Source::Unknown, None, None);
const SDTV: Quality = q(Source::Tv, Some(480), None);
const DVD: Quality = q(Source::Dvd, Some(480), None);

/// The quality a file's extension suggests when the name says nothing.
fn by_extension(name: &str) -> Quality {
    let Some((_, ext)) = name.rsplit_once('.') else { return UNKNOWN };
    match ext.to_ascii_lowercase().as_str() {
        "m4v" | "3gp" | "nsv" | "ty" | "strm" | "rm" | "rmvb" | "m3u" | "ifo" | "mov" | "qt" | "divx" | "xvid"
        | "bivx" | "nrg" | "pva" | "wmv" | "asf" | "asx" | "ogm" | "ogv" | "m2v" | "avi" | "bin" | "dat" | "dvr-ms"
        | "mpg" | "mpeg" | "mp4" | "avc" | "vp3" | "svq3" | "nuv" | "viv" | "dv" | "fli" | "flv" | "wpl" => SDTV,
        "img" | "iso" | "vob" => DVD,
        "mkv" | "mk3d" | "ts" | "wtv" => q(Source::Tv, Some(720), None),
        "m2ts" => q(Source::Bluray, Some(720), None),
        _ => UNKNOWN,
    }
}

fn resolution(name: &str) -> Option<u32> {
    if let Some(m) = RESOLUTION.find(name) {
        for (group, lines) in
            [("R360p", 360), ("R480p", 480), ("R540p", 540), ("R576p", 576), ("R720p", 720), ("R1080p", 1080)]
        {
            if m.named_group(group).is_some() {
                return Some(lines);
            }
        }
        if m.named_group("R2160p").is_some() {
            return Some(2160);
        }
    }
    ALTERNATIVE_RESOLUTION.is_match(name).then_some(2160)
}

fn contains(name: &str, word: &str) -> bool {
    name.to_ascii_lowercase().contains(&word.to_ascii_lowercase())
}

/// 360p, 480p and 540p are all standard definition, kept as 480.
fn sd(resolution: Option<u32>) -> bool {
    matches!(resolution, Some(360 | 480 | 540 | 576))
}

/// The quality of a release name.
pub fn quality(name: &str) -> Quality {
    let name = name.trim();
    if name.is_empty() {
        return UNKNOWN;
    }
    let found = from_name(name);
    if found == UNKNOWN { by_extension(name) } else { found }
}

fn from_name(name: &str) -> Quality {
    use Source::*;
    let normalized = name.replace('_', " ");
    let n = normalized.trim();
    let source = SOURCE.find_iter(n).last();
    let res = resolution(n);
    let codec = CODEC.find(n);
    let codec_is = |group: &str| codec.as_ref().is_some_and(|m| m.named_group(group).is_some());
    let remux = REMUX.is_match(n) || GERMAN_REMUX.is_match(n);
    let br_disk = BR_DISK.is_match(n);

    if RAW_HD.is_match(n) && !br_disk {
        return q(Tv, Some(1080), Some(Modifier::Rawhd));
    }

    if let Some(m) = &source {
        let is = |group: &str| m.named_group(group).is_some();
        if is("bluray") {
            if br_disk {
                return q(Bluray, Some(1080), Some(Modifier::Brdisk));
            }
            if codec_is("xvid") || codec_is("divx") {
                return q(Bluray, Some(480), None);
            }
            return match res {
                Some(2160) => q(Bluray, Some(2160), remux.then_some(Modifier::Remux)),
                Some(1080) => q(Bluray, Some(1080), remux.then_some(Modifier::Remux)),
                Some(720) => q(Bluray, Some(720), None),
                Some(576) => q(Bluray, Some(576), None),
                Some(360 | 480 | 540) => q(Bluray, Some(480), None),
                // A remux without a resolution is 1080p, not 720p.
                _ if remux => q(Bluray, Some(1080), Some(Modifier::Remux)),
                _ => q(Bluray, Some(720), None),
            };
        }
        if is("webdl") {
            return match res {
                Some(2160 | 1080 | 720) => q(Webdl, res, None),
                _ if name.contains("[WEBDL]") => q(Webdl, Some(720), None),
                _ => q(Webdl, Some(480), None),
            };
        }
        if is("webrip") {
            return match res {
                Some(2160 | 1080 | 720) => q(Webrip, res, None),
                _ => q(Webrip, Some(480), None),
            };
        }
        if is("scr") {
            return q(Dvd, Some(480), Some(Modifier::Screener));
        }
        if is("cam") {
            return q(Cam, None, None);
        }
        if is("ts") {
            return q(Telesync, res, None);
        }
        if is("tc") {
            return q(Telecine, None, None);
        }
        if is("wp") {
            return q(Workprint, None, None);
        }
        if is("regional") {
            return q(Dvd, Some(480), Some(Modifier::Regional));
        }
        if is("hdtv") {
            if MPEG2.is_match(n) {
                return q(Tv, Some(1080), Some(Modifier::Rawhd));
            }
            return match res {
                Some(2160 | 1080 | 720) => q(Tv, res, None),
                _ if name.contains("[HDTV]") => q(Tv, Some(720), None),
                _ => SDTV,
            };
        }
        if is("bdrip") || is("brrip") {
            return match res {
                Some(720 | 1080 | 2160 | 576) => q(Bluray, res, None),
                _ => q(Bluray, Some(480), None),
            };
        }
        if is("dvdr") {
            return q(Dvd, Some(480), Some(Modifier::Remux));
        }
        if is("dvd") {
            return DVD;
        }
        if is("pdtv") || is("sdtv") || is("dsr") || is("tvrip") {
            if res == Some(1080) || contains(n, "1080p") {
                return q(Tv, Some(1080), None);
            }
            if res == Some(720) || contains(n, "720p") || HIGH_DEF_PDTV.is_match(n) {
                return q(Tv, Some(720), None);
            }
            return SDTV;
        }
    }

    if source.is_none() && remux {
        match res {
            Some(480) => return q(Bluray, Some(480), None),
            Some(720) => return q(Bluray, Some(720), None),
            Some(2160 | 1080) => return q(Bluray, res, Some(Modifier::Remux)),
            _ => {}
        }
    }

    if ANIME_BLURAY.is_match(n) {
        if sd(res) || contains(n, "480p") {
            return DVD;
        }
        if res == Some(1080) || contains(n, "1080p") {
            return q(Bluray, Some(1080), remux.then_some(Modifier::Remux));
        }
        if res == Some(2160) || contains(n, "2160p") {
            return q(Bluray, Some(2160), remux.then_some(Modifier::Remux));
        }
        if remux && res != Some(720) {
            return q(Bluray, Some(1080), Some(Modifier::Remux));
        }
        return q(Bluray, Some(720), None);
    }

    if ANIME_WEBDL.is_match(n) {
        if sd(res) || contains(n, "480p") {
            return q(Webdl, Some(480), None);
        }
        if res == Some(1080) || contains(n, "1080p") {
            return q(Webdl, Some(1080), None);
        }
        if res == Some(2160) || contains(n, "2160p") {
            return q(Webdl, Some(2160), None);
        }
        return q(Webdl, Some(720), None);
    }

    if let Some(lines) = res {
        let (source, modifier) =
            if remux { (Bluray, Some(Modifier::Remux)) } else { (by_extension(name).source, None) };
        let lines = if sd(Some(lines)) { 480 } else { lines };
        return if source == Unknown { q(Tv, Some(lines), None) } else { q(source, Some(lines), modifier) };
    }

    if codec_is("x264") {
        return SDTV;
    }
    if contains(n, "848x480") {
        return if n.contains("dvd") {
            DVD
        } else if contains(n, "bluray") {
            q(Bluray, Some(480), None)
        } else {
            SDTV
        };
    }
    for (size, lines) in [("1280x720", 720), ("1920x1080", 1080)] {
        if contains(n, size) {
            return q(if contains(n, "bluray") { Bluray } else { Tv }, Some(lines), None);
        }
    }
    for (word, lines) in [("bluray720p", 720), ("bluray1080p", 1080), ("bluray2160p", 2160)] {
        if contains(n, word) {
            return q(Bluray, Some(lines), None);
        }
    }
    if let Some(m) = OTHER_SOURCE.find(n) {
        if m.named_group("sdtv").is_some() {
            return SDTV;
        }
        if m.named_group("hdtv").is_some() {
            return q(Tv, Some(720), None);
        }
    }
    UNKNOWN
}
