use std::{
    fs,
    process::Command,
};

use super::{
    check_arguments,
    positions,
};

#[test]
fn rejects_inline_struct_construction_in_function_and_method_arguments() {
    for statement in [
        "result.push(Output { id: value.id.clone() });",
        "consume(Output { id });",
        "consume(&Output { id });",
        "consume((Output { id }));",
        "consume(Output { id }.into());",
        "consume(Some(Output { id }));",
        "consume((Output { id }, id));",
        "consume(if condition { Output { id } } else { fallback });",
        "consume({ Output { id } });",
        "consume(vec![Output { id }]);",
        "consume(identity!(Output { id }));",
        "assert!(consume(Output { id }));",
        "consume(matches!(Output { id }, Output { .. }));",
        "consume(matches!(value, Output { id } if allowed(Output { id })));",
        "result.push(Output { ..previous });",
        "Ok(Self { id })",
    ] {
        let source = format!("fn example() {{ {statement} }}");
        assert_eq!(positions(&source).unwrap().len(), 1, "{statement}");
    }
}

#[test]
fn permits_named_values_and_non_argument_construction() {
    for statement in [
        "let output = Output { id: value.id.clone() }; result.push(output);",
        "let output = Output { nested: Nested { id } }; consume(&output);",
        "return Output { id };",
        "items.map(|value| Output { id: value.id });",
        "items.map(|value| { let output = Output { id: value.id }; output });",
        "consume({ let output = Output { id }; output });",
        "spawn(async move { Output { id } });",
        "consume(build());",
        "consume(42, \"Output { id }\", value.field);",
        "consume(matches!(value, Output { .. }));",
        "consume(assert_matches!(value, Output { id }));",
    ] {
        let source = format!("fn example() {{ {statement} }}");
        assert!(positions(&source).unwrap().is_empty(), "{statement}");
    }
}

#[test]
fn inspects_calls_inside_closures_async_blocks_and_initializers() {
    for statement in [
        "items.map(|value| consume(Output { id: value.id }));",
        "spawn(async move { consume(Output { id }); });",
        "consume({ let output = build(Output { id }); output });",
        "let output = Outer { nested: Box::new(Nested { id }) };",
    ] {
        let source = format!("fn example() {{ {statement} }}");
        assert_eq!(positions(&source).unwrap().len(), 1, "{statement}");
    }
}

#[test]
fn checks_test_code_and_reports_each_literal_once_at_its_source_position() {
    let source = "#[test]\nfn example() {\n    result.push(Some(Output { id }));\n}";
    assert_eq!(positions(source).unwrap(), [(3, 22)].into());
    let unicode = "fn example() { consume(\"α\", Output { id }); }";
    assert_eq!(positions(unicode).unwrap(), [(1, 29)].into());
}

#[test]
fn checks_git_scoped_rust_files_without_changing_them() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    fs::create_dir(root.join("ignored")).unwrap();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    let files = [
        ("tracked.rs", "fn example() { consume(Output { id }); }"),
        (
            "untracked.rs",
            "fn example() { result.push(Output { id }); }",
        ),
        (
            "named.rs",
            "fn example() { let output = Output { id }; consume(output); }",
        ),
        (
            "ignored/generated.rs",
            "fn example() { consume(Output { id }); }",
        ),
        ("other.txt", "consume(Output { id });"),
    ];
    for (path, source) in files {
        fs::write(root.join(path), source).unwrap();
    }
    fs::write(root.join("deleted.rs"), "fn example() {}").unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["add", "tracked.rs", "deleted.rs"])
            .status()
            .unwrap()
            .success()
    );
    fs::remove_file(root.join("deleted.rs")).unwrap();
    assert_eq!(check_arguments(root).unwrap(), 2);
    for (path, source) in files {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), source);
    }
}
