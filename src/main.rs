use std::env::args;

use libtlafmt::ParsedFile;

fn main() {
    let input = std::fs::read_to_string(args().nth(1).expect("no filename passed as argument"))
        .expect("must read file path");

    ParsedFile::new(input.as_str())
        .unwrap()
        .format(std::io::stdout().lock())
        .expect("format error");
}
