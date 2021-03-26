// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

/// Generate `pipewire-rs/pipewire/src/auto/keys.rs` from `pipewire/src/pipewire/keys.h`
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::PathBuf,
};

use anyhow::Result;
use regex::Regex;
use structopt::StructOpt;

const HEADER: &str = r#"// This file was generated using:
// cargo run --manifest-path tools/Cargo.toml  -- ../pipewire/src/pipewire/keys.h > pipewire/src/auto/keys.rs
// DO NOT EDIT
"#;

fn trim_comment(comment: &str) -> String {
    let comment = comment.trim();
    // single line comments are terminated with "*/""
    if let Some(comment) = comment.strip_suffix("*/") {
        comment.trim().to_string()
    } else {
        comment.to_string()
    }
}

fn parse<T: Read>(input: T) -> Result<Vec<Key>> {
    let reader = BufReader::new(input);
    let mut keys = Vec::new();
    let mut deprecated = false;

    let reg_define =
        Regex::new(r#"^#define PW_KEY_([A-Z_0-9]+)[[:space:]]+"(.*)"[[:space:]]*/\*\*<(.*)$"#)?;
    let reg_comment = Regex::new(r#"\* (.*)"#)?;

    for l in reader.lines() {
        let l = l?;
        let l = l.trim();
        if l.is_empty() {
            continue;
        }

        if l == "#ifdef PW_ENABLE_DEPRECATED" {
            deprecated = true;
            continue;
        } else if l == "#endif /* PW_ENABLE_DEPRECATED */" {
            deprecated = false;
            continue;
        } else if l.starts_with("/*") {
            // skip section comment such as "/* config */"
            continue;
        }

        if deprecated {
            // ignore everythin in the deprecated block
            continue;
        }

        if let Some(capture) = reg_define.captures(l) {
            // new key
            let rust_symb = capture
                .get(1)
                .expect("failed to extract Rust identifier")
                .as_str();

            let comment = capture.get(3).expect("failed to extract comment").as_str();
            let comment = trim_comment(comment);

            let key = Key::new(rust_symb, &comment);
            keys.push(key);
            continue;
        } else if let Some(capture) = reg_comment.captures(l) {
            // expand multi-lines comment of the last key
            if let Some(mut key) = keys.pop() {
                let comment = capture.get(1).expect("failed to extract comment").as_str();
                let comment = trim_comment(comment);
                key.comment.push_str(&format!(" {}", comment));

                keys.push(key);
            }
        } else if l == "#ifdef PW_ENABLE_DEPRECATED" {
            deprecated = true;
        } else if l == "#endif /* PW_ENABLE_DEPRECATED */" {
            deprecated = false;
        }
    }

    Ok(keys)
}

fn generate_rust(keys: &[Key]) -> String {
    let mut res = String::new();

    for key in keys.iter() {
        res.push_str(&format!(
            "key_constant!({}, {},
    /// {}
);
",
            key.rust_symb, key.c_symb, key.comment
        ));
    }

    res
}

#[derive(Debug, PartialEq)]
struct Key {
    rust_symb: String,
    c_symb: String,
    comment: String,
}

impl Key {
    fn new(rust_symb: &str, comment: &str) -> Self {
        Self {
            rust_symb: rust_symb.to_string(),
            c_symb: format!("PW_KEY_{}", rust_symb),
            comment: comment.to_string(),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "gen-keys", about = "Generate keys constant")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let input = File::open(&opt.input)?;
    let keys = parse(input)?;
    let output = generate_rust(&keys);

    print!("{}\n{}", HEADER, output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_one_line() {
        let input = r#"
#define PW_KEY_PROTOCOL			"pipewire.protocol"	/**< protocol used for connection */
/** Section */
#define PW_KEY_SEC_PID			"pipewire.sec.pid"	/**< Client pid, set by protocol */
#define PW_KEY_WINDOW_X11_DISPLAY	"window.x11.display"	/**< the X11 display string. Ex. ":0.0" */
        "#;

        let input = Cursor::new(input);
        let keys = parse(input).expect("parsing failed");

        assert_eq!(
            keys,
            vec![
                Key::new("PROTOCOL", "protocol used for connection"),
                Key::new("SEC_PID", "Client pid, set by protocol"),
                Key::new("WINDOW_X11_DISPLAY", "the X11 display string. Ex. \":0.0\""),
            ]
        );
    }

    #[test]
    fn test_parse_multi_lines() {
        let input = r#"
#define PW_KEY_CLIENT_ACCESS		"pipewire.client.access"/**< how the client wants to be access
								  *  controlled */
/* Section */
#define PW_KEY_REMOTE_NAME		"remote.name"		/**< The name of the remote to connect to,
								  *  default pipewire-0, overwritten by
								  *  env(PIPEWIRE_REMOTE) */
        "#;

        let input = Cursor::new(input);
        let keys = parse(input).expect("parsing failed");

        assert_eq!(
            keys,
            vec![
                Key::new("CLIENT_ACCESS", "how the client wants to be access controlled"),
                Key::new("REMOTE_NAME", "The name of the remote to connect to, default pipewire-0, overwritten by env(PIPEWIRE_REMOTE)")
            ]
        );
    }

    #[test]
    fn test_parse_deprecated() {
        let input = r#"
#define PW_KEY_PROTOCOL			"pipewire.protocol"	/**< protocol used for connection */

#ifdef PW_ENABLE_DEPRECATED
#define PW_KEY_PRIORITY_MASTER		"priority.master"	/**< deprecated */
#endif /* PW_ENABLE_DEPRECATED */

#define PW_KEY_SEC_PID			"pipewire.sec.pid"	/**< Client pid, set by protocol */
        "#;

        let input = Cursor::new(input);
        let keys = parse(input).expect("parsing failed");

        assert_eq!(
            keys,
            vec![
                Key::new("PROTOCOL", "protocol used for connection"),
                Key::new("SEC_PID", "Client pid, set by protocol"),
            ]
        );
    }

    #[test]
    fn test_generate() {
        let keys = vec![
            Key::new("PROTOCOL", "protocol used for connection"),
            Key::new("SEC_PID", "Client pid, set by protocol"),
        ];
        let expected = r#"key_constant!(PROTOCOL, PW_KEY_PROTOCOL,
    /// protocol used for connection
);
key_constant!(SEC_PID, PW_KEY_SEC_PID,
    /// Client pid, set by protocol
);
"#;

        assert_eq!(generate_rust(&keys), expected);
    }
}
