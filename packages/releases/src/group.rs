//! The release group: who made the release, read as Sonarr and Radarr read
//! it (their `ReleaseGroupParser`, the lists of both). Its patterns run on
//! PTT's ECMAScript engine, which takes .NET's look-behinds and named groups
//! as they are.

use std::sync::LazyLock;

use parse_torrent_title::Regex;

fn re(source: &str, flags: &str) -> Regex {
    Regex::new(source, flags).expect("a release group pattern")
}

/// The usual endings of a release name's file (or of the download that
/// holds it), dropped before the group is read.
const EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "m4v", "ts", "m2ts", "wmv", "mov", "mpg", "mpeg", "webm", "flv", "divx", "xvid", "iso", "img",
    "nzb", "torrent", "rar", "zip", "srt", "ass", "sub",
];

/// `[www.site.com] - Name…` and the like before the name.
static WEBSITE_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?:(?:\[|\()\s*)?(?:www\.)?[-a-z0-9-]{1,256}\.(?<!Naruto-Kun\.)(?:[a-z]{2,6}\.[a-z]{2,6}|xn--[a-z0-9-]{4,}|[a-z]{2,})\b(?:\s*(?:\]|\))|[ -]{2,})[ -]*",
        "i",
    )
});

/// The tags torrent sites put after a name (`[ettv]`).
static TORRENT_SUFFIX: LazyLock<Regex> = LazyLock::new(|| re(r"\[(?:ettv|rartv|rarbg|cttv|publichd)\]$", "i"));

/// An anime release opens with its fansub group: `[SubsPlease] Show - 01`.
static ANIME_GROUP: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:\[(?<subgroup>(?!\s).+?(?<!\s))\](?:_|-|\s|\.)?)", "i"));

/// What uploaders and indexers add after the group (`-Rakuv`, `-postbot`),
/// and everything before an episode code's own group.
static CLEAN: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(.*?[-._ ](S\d+E\d+)[-._ ])|(-(RP|1|NZBGeek|\[N-Z-B\]|Obfuscated|Obfuscation|Scrambled|sample|Pre|postbot|xpost|Rakuv[a-z0-9]*|WhiteRev|BUYMORE|AsRequested|AlternativeToRequested|GEROV|Z0iDS3N|Chamele0n|4P|4Planet|AlteZachen|RePACKPOST))+$",
        "gi",
    )
});

/// Groups whose releases end with `Group)` or `Group]`.
static BRACKETED: LazyLock<Regex> = LazyLock::new(|| {
    re(
        concat!(
            r"(?<=[._ \[])(?<releasegroup>(Silence|afm72|Panda|Ghost|MONOLITH|Tigole|Joy|ImE|UTR|t3nzin|Anime Time",
            r"|Project Angel|Hakata Ramen|HONE|GiLG|Vyndros|SEV|Garshasp|Kappa|Natty|RCVR|SAMPA|YOGI|r00t|EDGE2020",
            r"|RZeroX|FreetheFish|Anna|Bandi|Qman|theincognito|HDO|DusIctv|DHD|CtrlHD|-ZR-|ADC|XZVN|RH|Kametsu|Celdra)",
            r"(?=\]|\)))",
        ),
        "gi",
    )
});

/// Groups whose names hold what would otherwise end them (`Fight-BB`, `E.N.D`).
static EXACT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        concat!(
            r"\b(?<releasegroup>Fight-BB|VARYG|E\.N\.D|KRaLiMaRKo|BluDragon|DarQ|KCRT|BEN[_. ]THE[_. ]MEN|TAoE|QxR",
            r"|Vialle|D-Z0N3|Koten_Gars|ZØNEHD|HQMUX|YIFY|YTS(?:\.(?:MX|LT|AG))?|TMd|Eml HDTeam|LMain|126811)\b",
        ),
        "gi",
    )
});

/// `-Group` at the end, a group of two words joined by a hyphen
/// (`D-Z0N3`) included, but not a codec's, an audio format's or a
/// resolution's hyphen; or `[Group]` at the very end.
static GROUP: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"-(?<releasegroup>[A-Za-zÀ-ÖØ-öø-ÿ0-9]+(?<part2>-[A-Za-zÀ-ÖØ-öø-ÿ0-9]+)?(?!.+?(?:HDTV|SDTV|480p|576p|720p|1080p|2160p)))(?<!(?:HDTV|SDTV|WEB-DL|Blu-Ray|480p|576p|720p|1080p|2160p|WEB-Rip|DTS-HD|DTS-X|DTS-MA|DTS-ES|-ES|-EN|-CAT|-ENG|-JAP|-GER|-FRA|-FRE|-ITA|-HDRip|\d{1,2}-bit|[ ._]\d{4}-\d{2}|-\d{2}|tmdb(?:id)?-\d+|tt\d{7,8})(?:\k<part2>)?)(?:\b|[-._ ]|$)|[-._ ]\[(?<releasegroup>[A-Za-zÀ-ÖØ-öø-ÿ0-9]+)\]$",
        "gi",
    )
});

/// A season or episode code, or a CRC, read as a group by mistake.
static INVALID: LazyLock<Regex> = LazyLock::new(|| re(r"^([se]\d+|[0-9a-f]{8})$", "i"));

/// The name without its file's extension.
pub(crate) fn without_extension(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((rest, ext)) if EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) => rest,
        _ => name,
    }
}

fn last<'t>(regex: &Regex, text: &'t str) -> Option<&'t str> {
    regex.find_iter(text).filter_map(|m| m.named_group("releasegroup").map(|r| &text[r])).last()
}

/// The release group a name ends with (or an anime's opens with).
pub fn release_group(name: &str) -> Option<String> {
    let name = without_extension(name.trim());
    let name = WEBSITE_PREFIX.replace(name, "");
    let name = TORRENT_SUFFIX.replace(&name, "");
    if let Some(m) = ANIME_GROUP.find(&name) {
        return m.named_group("subgroup").map(|r| name[r].to_owned());
    }
    let name = CLEAN.replace_all(&name, "");
    if let Some(group) = last(&BRACKETED, &name).or_else(|| last(&EXACT, &name)) {
        return Some(group.to_owned());
    }
    let group = last(&GROUP, &name)?;
    if group.parse::<i64>().is_ok() || INVALID.is_match(group) {
        return None;
    }
    Some(group.to_owned())
}
