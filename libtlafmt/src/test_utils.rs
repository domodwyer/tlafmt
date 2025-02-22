#![cfg(test)]

/// Parse and format the macro string argument and generate a insta snapshot
/// assertion against it in the name of the caller.
#[macro_export]
macro_rules! assert_rewrite {
    ($input:expr) => {{
        let mut buf = Vec::new();
        $crate::ParsedFile::new($input)
            .expect("parse AST")
            .format(&mut buf)
            .expect("format AST");

        let output = String::from_utf8(buf).expect("valid utf8");
        ::insta::assert_snapshot!(output);

        // Output must be idempotent.
        let mut buf = Vec::new();
        $crate::ParsedFile::new(&output)
            .expect("parse AST")
            .format(&mut buf)
            .expect("format AST");
        let output2 = String::from_utf8(buf).expect("valid utf8");
        ::pretty_assertions::assert_eq!(output, output2, "non-idempotent formatting");
    }};
}
