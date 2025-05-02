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
    family_filter: Option<Regex>,
    families: OnceCell<Vec<(PathBuf, Result<FamilyProto, ParseError>)>>,
    languages: OnceCell<Vec<Result<LanguageProto, ParseError>>>,
    family_by_font_file: OnceCell<HashMap<String, usize>>,
}

impl GoogleFonts {
    pub fn new(p: PathBuf, family_filter: Option<Regex>) -> Self {
        Self {
            repo_dir: p,
            family_filter,
            families: OnceCell::new(),
            languages: OnceCell::new(),
            family_by_font_file: OnceCell::new(),
        }
    }

    pub fn families(&self) -> &[(PathBuf, Result<FamilyProto, ParseError>)] {
        self.families
            .get_or_init(|| iter_families(&self.repo_dir, self.family_filter.as_ref()).collect())
            .as_slice()
    }

    pub fn languages(&self) -> &[Result<LanguageProto, ParseError>] {
        self.languages
            .get_or_init(|| iter_languages(&self.repo_dir).collect())
            .as_slice()
    }

    pub fn language(&self, lang_id: &str) -> Option<&LanguageProto> {
        self.languages()
            .iter()
            .filter_map(|l| l.as_ref().ok())
            .find(|l| l.id() == lang_id)
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

    /// Our best guess at the primary language for this family
    ///
    /// Meant to be a good choice for things like rendering a sample string
    pub fn primary_language(&self, family: &FamilyProto) -> &LanguageProto {
        // Probe primary lang, primary script, then default baselessly to latin
        let mut primary_language: Option<&LanguageProto> = None;
        eprintln!("{family:#?}");
        if primary_language.is_none() && family.has_primary_language() {
            if let Some(lang) = self.language(family.primary_language()) {
                primary_language = Some(lang);
                eprintln!(
                    "Use primary_language {} for {}",
                    family.primary_language(),
                    family.name()
                );
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
            let lang = self
                .languages()
                .iter()
                .filter_map(|r| r.as_ref().ok())
                .filter(|l| l.has_script() && l.script() == family.primary_script())
                .reduce(|acc, e| {
                    if acc.population() > e.population() {
                        acc
                    } else {
                        e
                    }
                });
            if let Some(lang) = lang {
                primary_language = Some(lang);
                eprintln!(
                    "Use {}, most populous lang for primary_script {} for {}",
                    family.primary_script(),
                    family.primary_language(),
                    family.name()
                );
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
            eprintln!(
                "Use primary_language {:?} for {}",
                primary_language,
                family.name()
            );
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
}
