#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use libtlafmt::*;

fuzz_target!(|data: &str| -> Corpus {
    let file = format!(
        r"
---- Bananas ----
{}
=================
",
        data
    );

    let p = match ParsedFile::new(&file) {
        Ok(p) => p,
        Err(_) => return Corpus::Reject,
    };

    let mut out = Vec::new();
    let _ = p.format(&mut out);

    Corpus::Keep
});
