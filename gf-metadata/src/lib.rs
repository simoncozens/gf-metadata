mod fonts_public;

pub use fonts_public::*;
use protobuf::text_format::ParseError;
use regex::Regex;

pub fn parse_from_str(s: &str) -> Result<FamilyProto, ParseError> {
    if s.contains("position") {
        let re = Regex::new(r"(?m)position\s+\{[^}]*\}").expect("Valid re");
        let s = re.replace_all(s, "");
        protobuf::text_format::parse_from_str(&s)
    } else {
        protobuf::text_format::parse_from_str(s)
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
    fn parse_roboto_metadata() {
        parse_from_str(&testdata_file_content("roboto-metadata.pb")).unwrap();
    }

    #[test]
    fn parse_wix_metadata() {
        // Has the undocumented position field
        parse_from_str(&testdata_file_content("wixmadefortext-metadata.pb")).unwrap();
    }
}
