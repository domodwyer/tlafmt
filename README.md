# tlafmt

A formatter for [TLA+] specs.

It looks like this:

```tla
Add(t, k, v) == \* Using transaction t, add value v to the store under key k.
    /\ t \in tx
    /\ snapshotStore[t][k] = NoVal
    /\ snapshotStore' = [snapshotStore EXCEPT ![t][k] = v]
    /\ written' = [written EXCEPT ![t] = @ \union {k}]
    /\ UNCHANGED << tx, missed, store >>
```

Some TLA specs are hand-crafted works of art - but not all of them - this tool
is for those ugly specs!

* Optimises for _easily readable_ output.
* Consistent formatting across different teams and authors.
* Helpful but not forceful - small tweaks, not big rewrites!
* _Fast_ rendering - suitable for "format on save" in interactive editors.

## Install

Download pre-built binaries from the [releases page].

Or if you have Rust installed, compile it yourself: `cargo install tlafmt`.

## Usage

Format a file and print the formatted result to stdout:

```shellsession
% tlafmt bananas.tla
```

Or overwrite the input file with the formatted result:

```shellsession
% tlafmt --in-place bananas.tla
```

To check if a file is formatted and return an error if it isn't, use `--check`:

```shellsession
% tlafmt --check bananas.tla
# Exits code 3 for unformatted code.
```

Check out the `--help` text too.

## Style

This formatter doesn't aim to enforce a prescriptive, universal style across the
whole spec. Instead it aims to improve consistency by making small changes,
respecting the general style of the input spec.

Formatting is being iterated on - please [report] any undesirable rendering as
an issue.

[TLA+]: https://lamport.azurewebsites.net/tla/tla.html
[releases page]: https://github.com/domodwyer/tlafmt/releases/latest
[report]: https://github.com/domodwyer/tlafmt/issues/new
