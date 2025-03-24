use std::ops::{Add, Sub};

pub(crate) const INDENT_STR: &str = "    ";

/// A fixed indentation level.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) struct Indent(u8);

impl Indent {
    pub(crate) const ZERO: Self = Self::new(0);

    pub(crate) const fn new(v: u8) -> Self {
        Self(v)
    }
    pub(crate) const fn get(&self) -> u8 {
        self.0
    }
}

impl Add<u8> for Indent {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub for Indent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// An indentation decorator which inserts the specified indentation after every
/// newline.
///
/// NOTE: this includes after newlines embedded within a token, such as a Raw or
/// Comment token.
#[derive(Debug, Default)]
pub(crate) struct IndentDecorator<W> {
    out: W,

    /// The indentation depth to render for subsequent writes.
    depth: u8,

    /// True when the last byte wrote to `out` was a newline.
    last_char_newline: bool,
}

impl<W> IndentDecorator<W> {
    pub(crate) fn new(out: W) -> Self {
        Self {
            depth: 0,
            out,
            last_char_newline: false,
        }
    }

    pub(crate) fn set(&mut self, depth: Indent) {
        self.depth = depth.get();
    }
}

impl<W> std::io::Write for IndentDecorator<W>
where
    W: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for chunk in buf.split_inclusive(|&v| v == b'\n') {
            if self.last_char_newline {
                for _ in 0..self.depth {
                    self.out.write_all(INDENT_STR.as_bytes())?;
                }
            }

            self.out.write_all(chunk)?;
            self.last_char_newline = chunk.last().is_some_and(|v| *v == b'\n');
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_newline_in_buf() {
        let s = "\
Bananas
Are good?

Yes.
";

        let mut buf = Vec::new();
        let mut out = IndentDecorator::new(&mut buf);

        out.set(Indent::new(2));
        out.write_all(s.as_bytes()).unwrap();

        let got = String::from_utf8(buf).unwrap();
        assert_eq!(got, "Bananas\n        Are good?\n        \n        Yes.\n");
    }
}
