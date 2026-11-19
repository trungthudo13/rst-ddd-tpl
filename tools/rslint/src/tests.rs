use std::{
    fs,
    process::Command,
};

use super::{
    check_comments,
    comment_positions,
};

#[test]
fn rejects_line_inline_block_and_documentation_comments() {
    for source in [
        "// ordinary",
        "let n = 1; // inline",
        "/// outer documentation",
        "//! inner documentation",
        "/** outer block documentation */",
        "/*! inner block documentation */",
        "/* outer /* nested */ outer */",
        "/* unterminated",
    ] {
        assert_eq!(comment_positions(source).len(), 1, "{source}");
    }
}

#[test]
fn ignores_comment_markers_in_rust_literals_and_handles_lifetimes() {
    let source = r#####"
        let url = "https://example.com/*path*/";
        let escaped = "\"// still a string";
        let raw = r###"a quote " // /* still a string "###;
        let bytes = b"/* bytes */";
        let raw_bytes = br##"a quote " /* // still bytes "##;
        let c_string = c"// C string";
        let raw_c = cr##"a quote " /* // still a C string "##;
        let slash = '/';
        let byte_slash = b'/';
        let quote = '\'';
        fn identity<'a>(value: &'a str) -> &'a str { value }
        let r#type = 1;
    "#####;
    assert!(comment_positions(source).is_empty());
    assert_eq!(
        comment_positions(&format!("{source}\n// real comment")).len(),
        1
    );
}

#[test]
fn reports_unicode_columns_and_crlf_lines() {
    assert_eq!(
        comment_positions("αβ // line\r\n  /* block\n nested */"),
        [(1, 4), (2, 3)]
    );
}

#[test]
fn handles_bom_shebang_and_crate_attributes() {
    assert_eq!(
        comment_positions("\u{feff}#!/usr/bin/env rust-script\n// comment"),
        [(2, 1)]
    );
    assert_eq!(comment_positions("#![allow(dead_code)]\n// comment"), [(
        2, 1
    )]);
}

#[test]
fn checks_only_project_rust_files_without_modifying_them() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    fs::create_dir(root.join("nested")).unwrap();
    fs::create_dir(root.join("ignored")).unwrap();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    let files = [
        ("tracked.rs", "// tracked"),
        ("nested/untracked.rs", "/* untracked */"),
        ("new.rs", "//! documentation"),
        ("ignored/skip.rs", "// ignored"),
        ("README.md", "// not Rust"),
        ("migration.sql", "/* not Rust */"),
    ];
    for (path, source) in files {
        fs::write(root.join(path), source).unwrap();
    }
    fs::write(root.join("deleted.rs"), "// deleted").unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["add", "tracked.rs", "deleted.rs"])
            .status()
            .unwrap()
            .success()
    );
    fs::remove_file(root.join("deleted.rs")).unwrap();
    assert_eq!(check_comments(root).unwrap(), 3);
    for (path, source) in files {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), source);
    }
    fs::write(root.join("invalid.rs"), [0xff]).unwrap();
    assert!(check_comments(root).unwrap_err().to_string().contains("invalid.rs"));
}

#[test]
fn fails_when_the_repository_cannot_be_listed() {
    let directory = tempfile::tempdir().unwrap();
    assert!(check_comments(directory.path()).is_err());
}
