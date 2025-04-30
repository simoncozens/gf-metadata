use std::fs;

use gf_metadata::parse_from_str;
use home::home_dir;
use walkdir::WalkDir;

fn main() {
    let home = home_dir().expect("Must have a home dir");
    let mut fonts = home.clone();
    fonts.push("oss/fonts");

    let mut success = 0;
    let mut fail = 0;
    for entry in WalkDir::new(fonts).into_iter().filter(|p| {
        let Ok(p) = p else {
            return true;
        };
        p.file_name() == "METADATA.pb"
    }) {
        let Ok(d) = entry else {
            eprintln!("walk error: {entry:?}");
            fail+=1;
            continue;
        };
        if let Err(e) = parse_from_str(&fs::read_to_string(d.path()).expect("To read files!")) {
            eprintln!("Unable to read {:?}: {e:?}", d.path());
            fail+=1;
            continue;
        }
        success += 1;
    }
    eprintln!("Read {}/{} successfully", success, success + fail);
}
