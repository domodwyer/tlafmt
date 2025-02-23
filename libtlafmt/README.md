[![crates.io](https://img.shields.io/crates/v/libtlafmt.svg)](https://crates.io/crates/libtlafmt)
[![docs.rs](https://docs.rs/libtlafmt/badge.svg)](https://docs.rs/libtlafmt)

A library crate for formatting TLA+ specs.

This crate is the formatter implementation for [tlafmt].

## Inner Workings

Formatting a TLA file occurs in three phases within this crate:

1. The input file is parsed into an abstract syntax tree.
2. The AST is then lowered into a formatter-specific representation.
3. The format representation is rendered into output text.

Step (1) is performed when calling [`ParsedFile::new()`] to initialise a new
instance, and steps (2) and (3) are performed when [`ParsedFile::format()`]
is called, writing the output to a provided [`std::io::Write`] sink.

## Testing

Run the tests with:

```shellsession
% cargo test --workspace
```

Some tests make use of [cargo-insta].

In addition to unit tests, a corpus of example TLA specs[^corpus] are formatted
and a snapshot of their output generated[^snapshots] after each test run. These
snapshots have to be "accepted" using cargo-insta (if their change is desirable)
to cause future tests runs to pass.

[tlafmt]: https://github.com/domodwyer/tlafmt
[cargo-insta]: https://crates.io/crates/cargo-insta
[^corpus]: A small subset of the official TLA examples repo - see
    `libtlafmt/tests/corpus/` in the code repo.
[^snapshots]: See `libtlafmt/src/snapshots/` in the code repo.
