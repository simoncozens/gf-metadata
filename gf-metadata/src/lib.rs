mod fonts_public;
mod languages_public;

use std::{
    cell::OnceCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub use fonts_public::*;
pub use languages_public::{
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

pub fn read_language(s: &str) -> Result<LanguageProto, ParseError> {
    protobuf::text_format::parse_from_str(s)
}

fn exemplar_score(font: &FontProto) -> i32 {
    let mut score = 0;
    // prefer regular
    if font.style() == "normal" {
        score += 16;
    }

    // prefer closer to 400
    score -= (font.weight() - 400) / 100;

    // prefer variable
    if font.filename().contains("].") {
        score += 1;
    }

    score
}

pub fn exemplar(family: &FamilyProto) -> Option<&FontProto> {
    family.fonts.iter().reduce(|acc, e| {
        if exemplar_score(acc) >= exemplar_score(e) {
            acc
        } else {
            e
        }
    })
}

fn iter_families(root: &Path) -> impl Iterator<Item = (PathBuf, Result<FamilyProto, ParseError>)> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter(|d| d.file_name() == "METADATA.pb")
        .map(|d| {
            (
                d.path().to_path_buf(),
                read_family(&fs::read_to_string(d.path()).expect("To read files!")),
            )
        })
}

pub fn iter_languages(root: &Path) -> impl Iterator<Item = Result<LanguageProto, ParseError>> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter(|d| {
            d.path()
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("gflanguages/data/languages")
                && d.file_name().to_string_lossy().ends_with(".textproto")
        })
        .map(|d| read_language(&fs::read_to_string(d.path()).expect("To read files!")))
}

pub struct GoogleFonts {
    repo_dir: PathBuf,
    families: OnceCell<Vec<(PathBuf, Result<FamilyProto, ParseError>)>>,
    languages: OnceCell<Vec<Result<LanguageProto, ParseError>>>,
    family_by_font_file: OnceCell<HashMap<String, usize>>,
}

impl GoogleFonts {
    pub fn new(p: PathBuf) -> Self {
        Self {
            repo_dir: p,
            families: OnceCell::new(),
            languages: OnceCell::new(),
            family_by_font_file: OnceCell::new(),
        }
    }

    pub fn families(&self) -> &[(PathBuf, Result<FamilyProto, ParseError>)] {
        self.families
            .get_or_init(|| iter_families(&self.repo_dir).collect())
            .as_slice()
    }

    pub fn languages(&self) -> &[Result<LanguageProto, ParseError>] {
        self.languages
            .get_or_init(|| iter_languages(&self.repo_dir).collect())
            .as_slice()
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
        let Some((family_path, _)) = self.family(font) else {
            return None;
        };
        let mut font_file = family_path.parent().unwrap().to_path_buf();
        font_file.push(font.filename());
        if !font_file.exists() {
            eprintln!("No such file as {font_file:?}");
        }
        font_file.exists().then_some(font_file)
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
}
