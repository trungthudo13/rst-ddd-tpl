use std::{
    fs,
    process::Command,
};

use super::{
    check_shared_values,
    declarations,
};

#[test]
fn rejects_nested_test_and_associated_declarations_at_their_positions() {
    let source = "pub enum State { Ready }\nconst LIMIT: usize = 2;\ntrait Port { const SIZE: \
                  usize; }\nimpl Port for Value { const SIZE: usize = 2; }\n#[cfg(test)] mod \
                  tests { fn run() { enum Local {} const X: u8 = 0; } }";
    let declarations = declarations(source).unwrap();
    assert_eq!(declarations.len(), 6);
    assert_eq!(declarations[&(1, 5)], "State");
    assert_eq!(declarations[&(2, 1)], "LIMIT");
    assert_eq!(declarations[&(3, 14)], "SIZE");
    assert_eq!(declarations[&(4, 23)], "SIZE");
}

#[test]
fn allows_imports_aliases_const_functions_blocks_and_generics() {
    let source = r#"
        pub use shared::{State, LIMIT};
        type Local<T> = shared::Choice<T>;
        const fn size<const N: usize>() -> usize { (const { 2 }) + N }
        struct Buffer<const N: usize>([u8; N]);
        fn value() { let text = "enum State {} const LIMIT: u8 = 1;"; }
        macro_rules! buffer { () => { struct Value<const N: usize>; }; }
    "#;
    assert!(declarations(source).unwrap().is_empty());
}

#[test]
fn inspects_macro_arguments_and_templates_without_duplicate_reports() {
    for source in [
        "wrapper! { pub enum State { Ready } const LIMIT: usize = 1; }",
        "macro_rules! values { ($name:ident) => { enum $name { Ready } const LIMIT: usize = 1; }; \
         }",
        "macro_rules! values { ($($name:ident),*) => { $(enum $name { Ready } const LIMIT: usize \
         = 1;)* }; }",
        "fn run() { wrapper!({ enum State {} const LIMIT: usize = 1; }); }",
        "macro_rules! values { ($name:ident) => (enum $name { Ready } const LIMIT: usize = 1;); }",
        "macro_rules! values { ($name:ident) => [enum $name { Ready } const LIMIT: usize = 1;]; }",
        "fn run() { wrapper!(enum State {} const LIMIT: usize = 1; let value = 1;); }",
    ] {
        assert_eq!(declarations(source).unwrap().len(), 2, "{source}");
    }
}

#[test]
fn checks_layer_sources_including_untracked_files_without_rewriting() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    fs::write(root.join(".gitignore"), "src/generated/\n").unwrap();
    let files = [
        ("src/main.rs", "enum Host {}"),
        ("src/tests/example.rs", "const TEST: u8 = 1;"),
        ("apps/domain/src/lib.rs", "enum Domain {}"),
        ("apps/application/src/lib.rs", "enum Application {}"),
        ("apps/infrastructure/src/lib.rs", "enum Adapter {}"),
        (
            "apps/infrastructure/tests/adapter.rs",
            "enum AdapterTest {}",
        ),
        (
            "apps/shared/src/lib.rs",
            "enum Shared {} const VALUE: u8 = 1;",
        ),
        ("tools/rslint/src/main.rs", "enum Tool {}"),
        ("src/generated/ignored.rs", "enum Generated {}"),
        ("src/main.rs.bak", "enum Backup {}"),
    ];
    for (path, source) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["add", "src/main.rs"])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(check_shared_values(root).unwrap(), 6);
    for (path, source) in files {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), source);
    }
    fs::write(root.join("src/invalid.rs"), "enum").unwrap();
    assert!(check_shared_values(root).unwrap_err().to_string().contains("src/invalid.rs"));
}
