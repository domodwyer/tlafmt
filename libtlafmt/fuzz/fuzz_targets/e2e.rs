#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use libtlafmt::*;

fuzz_target!(|data: &str| -> Corpus {
    let p = match ParsedFile::new(data) {
        Ok(p) => p,
        Err(_) => return Corpus::Reject,
    };

    let mut out = Vec::new();
    let _ = p.format(&mut out);

    Corpus::Keep
});
