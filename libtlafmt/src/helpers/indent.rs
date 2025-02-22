pub(crate) const INDENT_STR: &str = "    ";

/// An indentation decorator which inserts the specified indentation after every
/// newline.
///
/// NOTE: this includes after newlines embedded within a token, such as a Raw or
/// Comment token.
#[derive(Debug, Default)]
pub(crate) struct IndentDecorator<W> {
    out: W,

    /// The indentation depth to render for subsequent writes.
    depth: usize,

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

    pub(crate) fn set(&mut self, depth: usize) {
        self.depth = depth;
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

        out.set(2);
        out.write_all(s.as_bytes()).unwrap();

        let got = String::from_utf8(buf).unwrap();
        assert_eq!(got, "Bananas\n        Are good?\n        \n        Yes.\n");
    }
}
