mod axes;
mod designers;
mod fonts_public;

use std::{
    cell::OnceCell,
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Error, ErrorKind},
    path::{Path, PathBuf},
    str::FromStr,
};

pub use axes::{AxisProto, FallbackProto};
pub use designers::{AvatarProto, DesignerInfoProto};
pub use fonts_public::*;
use google_fonts_languages::LANGUAGES;
pub use google_fonts_languages::{
    ExemplarCharsProto, LanguageProto, RegionProto, SampleTextProto, ScriptProto,
};
use protobuf::text_format::ParseError;
use regex::Regex;
use walkdir::WalkDir;

pub fn read_family(s: &str) -> Result<FamilyProto, ParseError> {
    if s.contains("position") {
        let re = Regex::new(r"(?m)position\s+\{[^}]*\}").expect("Valid re");
        let s = re.replace_all(s, "");
        protobuf::text_format::parse_from_str(&s)
    } else {
        protobuf::text_format::parse_from_str(s)
    }
}

fn exemplar_score(font: &FontProto, preferred_style: FontStyle, preferred_weight: i32) -> i32 {
    let mut score = 0;
    // prefer preferred_style
    if font.style() == preferred_style.style() {
        score += 16;
    }

    // prefer closer to preferred_weight
    score -= (font.weight() - preferred_weight).abs() / 100;

    // prefer more weight to less weight
    if font.weight() > preferred_weight {
        score += 1;
    }

    // prefer variable
    if font.filename().contains("].") {
        score += 2;
    }

    score
}

pub fn exemplar(family: &FamilyProto) -> Option<&FontProto> {
    fn score(font: &FontProto) -> i32 {
        exemplar_score(font, FontStyle::Normal, 400)
    }
    family
        .fonts
        .iter()
        .reduce(|acc, e| if score(acc) >= score(e) { acc } else { e })
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl FontStyle {
    fn style(&self) -> &str {
        match self {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
        }
    }
}

pub fn select_font(
    family: &FamilyProto,
    preferred_style: FontStyle,
    preferred_weight: i32,
) -> Option<&FontProto> {
    let score =
        |font: &FontProto| -> i32 { exemplar_score(font, preferred_style, preferred_weight) };
    family
        .fonts
        .iter()
        .reduce(|acc, e| if score(acc) >= score(e) { acc } else { e })
}

fn iter_families(
    root: &Path,
    filter: Option<&Regex>,
) -> impl Iterator<Item = (PathBuf, Result<FamilyProto, ParseError>)> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter(|d| d.file_name() == "METADATA.pb")
        .filter(move |d| {
            filter
                .map(|r| r.find(&d.path().to_string_lossy()).is_some())
                .unwrap_or(true)
        })
        .map(|d| {
            (
                d.path().to_path_buf(),
                read_family(&fs::read_to_string(d.path()).expect("To read files!")),
            )
        })
}

pub fn iter_languages(_root: &Path) -> impl Iterator<Item = Result<LanguageProto, ParseError>> {
    LANGUAGES.values().map(|l| Ok(*l.clone()))
}

pub fn read_tags(root: &Path) -> Result<Vec<Tag>, Error> {
    let mut tag_dir = root.to_path_buf();
    tag_dir.push("tags/all");
    let mut tags = Vec::new();
    for entry in fs::read_dir(&tag_dir).expect("To read tag dir") {
        let entry = entry.expect("To access tag dir entries");
        if entry
            .path()
            .extension()
            .expect("To have extensions")
            .to_str()
            .expect("utf-8")
            != "csv"
        {
            continue;
        }
        let fd = File::open(entry.path())?;
        let rdr = BufReader::new(fd);
        tags.extend(
            rdr.lines()
                .map(|s| s.expect("Valid tag lines"))
                .map(|s| Tag::from_str(&s).expect("Valid tag lines")),
        );
    }
    Ok(tags)
}

pub fn read_tag_metadata(root: &Path) -> Result<Vec<TagMetadata>, Error> {
    let mut tag_metadata_file = root.to_path_buf();
    tag_metadata_file.push("tags/tags_metadata.csv");
    let mut metadata = Vec::new();

    let fd = File::open(&tag_metadata_file)?;
    let rdr = BufReader::new(fd);
    metadata.extend(
        rdr.lines()
            .map(|s| s.expect("Valid tag lines"))
            .map(|s| TagMetadata::from_str(&s).expect("Valid tag metadata lines")),
    );

    Ok(metadata)
}

fn csv_values(s: &str) -> Vec<&str> {
    let mut s = s;
    let mut values = Vec::new();
    while !s.is_empty() {
        s = s.trim();
        let mut end_idx = None;
        if let Some(s) = s.strip_prefix('"') {
            end_idx = Some(s.find('"').expect("Close quote"));
        }
        end_idx = s[end_idx.unwrap_or_default()..]
            .find(',')
            .map(|v| v + end_idx.unwrap_or_default());
        if let Some(end_idx) = end_idx {
            let (value, rest) = s.split_at(end_idx);
            values.push(value.trim());
            s = &rest[1..];
        } else {
            values.push(s);
            s = "";
        }
    }
    values
}

#[derive(Clone, Debug)]
pub struct Tag {
    pub family: String,
    pub loc: String,
    pub tag: String,
    pub value: f32,
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values = csv_values(s);
        let (family, loc, tag, value) = match values[..] {
            [family, tag, value] => (family, "", tag, value),
            [family, loc, tag, value] => (family, loc, tag, value),
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unparseable tag")),
        };
        Ok(Tag {
            family: family.to_string(),
            loc: loc.to_string(),
            tag: tag.to_string(),
            value: f32::from_str(value)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid tag value"))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TagMetadata {
    pub tag: String,
    pub min_value: f32,
    pub max_value: f32,
    pub prompt_name: String,
}

impl FromStr for TagMetadata {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values = csv_values(s);
        let [tag, min, max, prompt_name] = values[..] else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Unparseable tag metadata, wrong number of values",
            ));
        };
        Ok(TagMetadata {
            tag: tag.into(),
            min_value: f32::from_str(min)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid min value"))?,
            max_value: f32::from_str(max)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid min value"))?,
            prompt_name: prompt_name.into(),
        })
    }
}

pub struct GoogleFonts {
    repo_dir: PathBuf,
    family_filter: Option<Regex>,
    families: OnceCell<Vec<(PathBuf, Result<FamilyProto, ParseError>)>>,
    family_by_font_file: OnceCell<HashMap<String, usize>>,
    tags: OnceCell<Result<Vec<Tag>, Error>>,
    tag_metadata: OnceCell<Result<Vec<TagMetadata>, Error>>,
}

impl GoogleFonts {
    pub fn new(p: PathBuf, family_filter: Option<Regex>) -> Self {
        Self {
            repo_dir: p,
            family_filter,
            families: OnceCell::new(),
            family_by_font_file: OnceCell::new(),
            tags: OnceCell::new(),
            tag_metadata: OnceCell::new(),
        }
    }

    pub fn tags(&self) -> Result<&[Tag], &Error> {
        self.tags
            .get_or_init(|| read_tags(&self.repo_dir))
            .as_ref()
            .map(|tags| tags.as_slice())
    }

    pub fn tag_metadata(&self) -> Result<&[TagMetadata], &Error> {
        self.tag_metadata
            .get_or_init(|| read_tag_metadata(&self.repo_dir))
            .as_ref()
            .map(|metadata| metadata.as_slice())
    }

    pub fn families(&self) -> &[(PathBuf, Result<FamilyProto, ParseError>)] {
        self.families
            .get_or_init(|| iter_families(&self.repo_dir, self.family_filter.as_ref()).collect())
            .as_slice()
    }

    pub fn language(&self, lang_id: &str) -> Option<&LanguageProto> {
        LANGUAGES.get(lang_id).map(|l| &**l)
    }

    fn family_by_font_file(&self) -> &HashMap<String, usize> {
        self.family_by_font_file.get_or_init(|| {
            self.families()
                .iter()
                .enumerate()
                .filter(|(_, (_, f))| f.is_ok())
                .flat_map(|(i, (_, f))| {
                    f.as_ref()
                        .unwrap()
                        .fonts
                        .iter()
                        .map(move |f| (f.filename().to_string(), i))
                })
                .collect()
        })
    }

    pub fn family(&self, font: &FontProto) -> Option<(&Path, &FamilyProto)> {
        self.family_by_font_file()
            .get(font.filename())
            .copied()
            .map(|i| {
                let (p, f) = &self.families()[i];
                (p.as_path(), f.as_ref().unwrap())
            })
    }

    pub fn find_font_binary(&self, font: &FontProto) -> Option<PathBuf> {
        let (family_path, _) = self.family(font)?;
        let mut font_file = family_path.parent().unwrap().to_path_buf();
        font_file.push(font.filename());
        if !font_file.exists() {
            eprintln!("No such file as {font_file:?}");
        }
        font_file.exists().then_some(font_file)
    }

    /// Our best guess at the primary language for this family
    ///
    /// Meant to be a good choice for things like rendering a sample string
    pub fn primary_language(&self, family: &FamilyProto) -> &LanguageProto {
        // Probe primary lang, primary script, then default baselessly to latin
        let mut primary_language: Option<&LanguageProto> = None;
        if primary_language.is_none() && family.has_primary_language() {
            if let Some(lang) = self.language(family.primary_language()) {
                primary_language = Some(lang);
            } else {
                eprintln!(
                    "{} specifies invalid primary_language {}",
                    family.name(),
                    family.primary_language()
                );
            }
        }
        if primary_language.is_none() && family.has_primary_script() {
            // If our script matches many languages pick the one with the highest population
            let lang = LANGUAGES
                .values()
                .filter(|l| l.script.is_some() && l.script() == family.primary_script())
                .reduce(|acc, e| {
                    if acc.population() > e.population() {
                        acc
                    } else {
                        e
                    }
                });
            if let Some(lang) = lang {
                primary_language = Some(lang);
            } else {
                eprintln!(
                    "{} specifies a primary_script that matches no languages {}",
                    family.name(),
                    family.primary_script()
                );
            }
        }
        if primary_language.is_none() {
            primary_language = self.language("en_Latn");
        }
        primary_language
            .unwrap_or_else(|| panic!("Not even our final fallback worked for {}", family.name()))
    }
}

#[cfg(test)]
mod tests {

    use std::fs;

    use super::*;

    fn testdata_dir() -> std::path::PathBuf {
        // cargo test seems to run in the project directory
        // VSCode test seems to run in the workspace directory
        // probe for the file we want in hopes of finding it regardless

        ["./resources/testdata", "../resources/testdata"]
            .iter()
            .map(std::path::PathBuf::from)
            .find(|pb| pb.exists())
            .unwrap()
    }

    fn testdata_file_content(relative_path: &str) -> String {
        let mut p = testdata_dir();
        p.push(relative_path);
        fs::read_to_string(p).unwrap()
    }

    #[test]
    fn roboto_exemplar() {
        let roboto = read_family(&testdata_file_content("roboto-metadata.pb")).unwrap();
        let exemplar = exemplar(&roboto).unwrap();
        assert_eq!("Roboto[wdth,wght].ttf", exemplar.filename());
    }

    #[test]
    fn wix_exemplar() {
        let roboto = read_family(&testdata_file_content("wixmadefortext-metadata.pb")).unwrap();
        let exemplar = exemplar(&roboto).unwrap();
        assert_eq!("WixMadeforText[wght].ttf", exemplar.filename());
    }

    #[test]
    fn parse_roboto_metadata() {
        read_family(&testdata_file_content("roboto-metadata.pb")).unwrap();
    }

    #[test]
    fn parse_wix_metadata() {
        // Has the undocumented position field
        read_family(&testdata_file_content("wixmadefortext-metadata.pb")).unwrap();
    }

    #[test]
    fn parse_primary_lang_script_metadata() {
        let family = read_family(&testdata_file_content("kosugimaru-metadata.pb")).unwrap();
        assert_eq!(
            ("Jpan", "Invalid"),
            (family.primary_script(), family.primary_language())
        );
    }

    #[test]
    fn parse_tag3() {
        Tag::from_str("Roboto Slab, /quant/stroke_width_min, 26.31").expect("To parse");
    }

    #[test]
    fn parse_tag4() {
        Tag::from_str("Roboto Slab, wght@100, /quant/stroke_width_min, 26.31").expect("To parse");
    }

    #[test]
    fn parse_tag_quoted() {
        Tag::from_str("Georama, \"ital,wght@1,100\", /quant/stroke_width_min, 16.97")
            .expect("To parse");
    }

    #[test]
    fn parse_tag_quoted2() {
        Tag::from_str("\"\",t,1").expect("To parse");
    }
}
