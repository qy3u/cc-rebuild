use std::path::{Path, PathBuf};

fn parse_include(line: &str) -> Option<String> {
    let line = line.trim_start();

    if !line.starts_with("#include") {
        return None;
    }

    let mut dep = String::new();
    let mut chars = line.chars();

    let mut sep = '<';
    let mut close = '>';

    loop {
        match chars.next() {
            Some(c) => {
                if c == '"' {
                    sep = '"';
                    close = '"';
                    break;
                }
                if c == '<' {
                    break;
                }
            }
            None => return None,
        };
    }

    loop {
        match chars.next() {
            Some(c) => {
                if c == close {
                    return Some(dep);
                }
                dep.push(c);
            }
            None => return None,
        }
    }
}

pub(crate) fn cpp_dep(content: &str) -> Vec<String> {
    let mut includes = Vec::new();

    for line in content.split("\n") {
        if line.contains("#include") {
            if let Some(d) = parse_include(line) {
                includes.push(d);
            }
        }
    }

    includes
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_include() {
        let stdio = r#"#include <stdio.h>"#;
        let custom = r#"#include "custom.h""#;
        let invalid = r#"#include "custom.h>"#;

        assert_eq!(parse_include(stdio), Some("stdio.h".to_string()));
        assert_eq!(parse_include(custom), Some("custom.h".to_string()));
        assert_eq!(parse_include(invalid), None);
    }

    #[test]
    fn test_parse_content() {
        let content = r#"
#include <stdio.h>
#include "somelib.h"
#include "culib.cuh"

int main() {
    return 0;
}
        "#;

        let deps = cpp_dep(content);
        assert_eq!(&deps[0], "stdio.h");
        assert_eq!(&deps[1], "somelib.h");
        assert_eq!(&deps[2], "culib.cuh");
    }
}
