use std::{
    fs,
    process::Command,
};

use super::{
    Catalog,
    Source,
    check_mappings,
    mapper_path,
    source_scope,
    test_files,
    violations,
};

fn count(
    path: &str,
    source: &str,
) -> usize {
    let source = Source {
        path: path.into(),
        scope: source_scope(path.as_ref()),
        syntax: syn::parse_file(source).unwrap(),
    };
    let mut catalog = Catalog::default();
    catalog.collect(&source.scope, &source.syntax.items);
    catalog.classify_runtime_fields();
    violations(&source, &catalog).len()
}

#[test]
fn unresolved_types_with_multiple_glob_imports_finish() {
    let source = r#"
        use crate::first::*;
        use crate::second::*;
        use crate::third::*;
        fn create(value: external::Unknown) -> external::Unknown { value }
    "#;
    assert_eq!(count("src/runtime.rs", source), 0);
}

#[test]
fn configuration_models_can_load_environment_values_without_a_mapper() {
    let source = r#"
        pub struct Settings { pub audience: String }
        impl Settings {
            fn from_env() -> Result<Self, Error> {
                let audience = std::env::var("AUDIENCE")?;
                let settings = Self { audience };
                Ok(settings)
            }
            fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
                let audience = lookup("AUDIENCE").unwrap();
                Self { audience }
            }
        }
    "#;
    assert_eq!(count("src/config.rs", source), 0);
}

#[test]
fn host_runtime_errors_can_be_handled_where_they_occur() {
    let source = r#"
        use shared::Error;
        async fn serve() -> Result<(), Error> {
            bind().await.map_err(|error| {
                Error::Unexpected(format!("listener initialization failed: {error}"))
            })?;
            Ok(())
        }
    "#;
    assert_eq!(count("src/server.rs", source), 0);
    for path in [
        "apps/application/src/service.rs",
        "src/api/handlers/example.rs",
        "src/internal/handlers/example.rs",
        "apps/infrastructure/src/adapter.rs",
        "apps/shared/src/service.rs",
    ] {
        assert_eq!(count(path, source), 1, "{path}");
    }
}

#[test]
fn shared_error_construction_is_not_a_model_conversion() {
    let sources: Vec<_> = [
        (
            "apps/shared/src/errors.rs",
            "pub enum Error { NotFound(String), StudyDoesNotExist(String) }",
        ),
        (
            "apps/shared/src/lib.rs",
            "mod errors; pub use errors::Error;",
        ),
        ("apps/domain/src/lib.rs", "pub use shared::Error;"),
        (
            "apps/application/src/handler.rs",
            r#"
                use shared::Error as Failure;
                use domain::Error;
                struct Command { id: String }
                struct View { id: String }
                mod other { pub enum Error { Missing(String) } }
                fn missing(command: Command) -> Failure {
                    Failure::NotFound(command.id)
                }
                fn missing_study(command: Command) -> Error {
                    Error::StudyDoesNotExist(command.id)
                }
                fn qualified(command: Command) -> shared::Error {
                    shared::Error::NotFound(command.id)
                }
                fn response(command: Command) -> View {
                    View { id: command.id }
                }
                fn other(command: Command) -> other::Error {
                    other::Error::Missing(command.id)
                }
            "#,
        ),
    ]
    .into_iter()
    .map(|(path, source)| {
        Source {
            path: path.into(),
            scope: source_scope(path.as_ref()),
            syntax: syn::parse_file(source).unwrap(),
        }
    })
    .collect();
    let mut catalog = Catalog::default();
    for source in &sources {
        catalog.collect(&source.scope, &source.syntax.items);
    }
    let actual = violations(&sources[3], &catalog);
    assert_eq!(actual.len(), 2, "{actual:?}");
}

#[test]
fn http_error_adapter_owns_its_response_and_shared_error_wrapper() {
    let source = r#"
        pub struct ApiError(pub shared::errors::Error);
        struct ErrorView { error: String }
        impl From<shared::errors::Error> for ApiError {
            fn from(error: shared::errors::Error) -> Self { Self(error) }
        }
        use axum::response::IntoResponse as Respond;
        impl Respond for ApiError {
            fn into_response(self) -> axum::response::Response {
                let view = ErrorView { error: format!("{:?}", self.0) };
                Json(view).into_response()
            }
        }
    "#;
    let path = "src/api/error.rs";
    assert_eq!(count(path, source), 0);
    assert_eq!(count("src/api/other_error.rs", source), 2);
    for implementation in [
        "impl From<String> for ApiError { fn from(value: String) -> Self { todo!() } }",
        "struct Input { id: String } struct Output { id: String } impl From<Input> for Output { \
         fn from(value: Input) -> Self { Self { id: value.id } } }",
    ] {
        assert_eq!(count(path, &format!("{source} {implementation}")), 1);
    }
}

#[test]
fn conversion_traits_are_rejected_regardless_of_qualification_or_aliases() {
    for source in [
        "impl From<Input> for Output { fn from(value: Input) -> Self { todo!() } }",
        "impl ::core::convert::TryFrom<Input> for Output { type Error = (); fn try_from(value: \
         Input) -> Result<Self, ()> { todo!() } }",
        "use std::convert::From as Convert; impl Convert<Input> for Output { fn from(value: \
         Input) -> Self { todo!() } }",
        "use core::convert as conversions; impl conversions::Into<Output> for Input { fn \
         into(self) -> Output { todo!() } }",
        "use std::{convert::{TryInto as Convert}}; impl Convert<Output> for Input { type Error = \
         (); fn try_into(self) -> Result<Output, ()> { todo!() } }",
        "impl r#From<Input> for Output { fn from(value: Input) -> Self { todo!() } }",
        "mod nested { use core::convert::From as Map; impl Map<Input> for Output { fn from(value: \
         Input) -> Self { todo!() } } }",
        "use axum::response::IntoResponse as Respond; impl Respond for Failure { fn \
         into_response(self) -> Response { todo!() } }",
        "impl std::str::FromStr for Value { type Err = (); fn from_str(value: &str) -> \
         Result<Self, ()> { todo!() } }",
    ] {
        assert_eq!(count("src/convert.rs", source), 1, "{source}");
    }
}

#[test]
fn only_a_physical_mapper_directory_qualifies() {
    let source = "impl From<Input> for Output { fn from(value: Input) -> Self { todo!() } }";
    for path in ["src/mappers/crf.rs", "src/api/mappers/crf/request.rs"] {
        assert_eq!(count(path, source), 0, "{path}");
    }
    for path in ["src/mappers.rs", "src/not_mappers/convert.rs", "src/mappers/../convert.rs"] {
        assert_eq!(count(path, source), 1, "{path}");
    }
    assert_eq!(count("src/mappers.rs", "pub mod crf; pub use crf::*;"), 0);
    assert_eq!(
        count("src/convert.rs", &format!("mod mappers {{ {source} }}")),
        1
    );
    assert!(!mapper_path("mappers/../src/convert.rs".as_ref()));
}

#[test]
fn derived_conversions_are_not_hidden_in_type_declarations() {
    for source in [
        "#[derive(derive_more::From)] pub struct Output(pub Input);",
        "use derive_more::TryInto as Convert; #[derive(Convert)] pub enum Input { Value(Output) }",
        "#[derive(thiserror::Error)] pub enum Failure { #[error(transparent)] Domain(#[from] \
         DomainError) }",
    ] {
        assert_eq!(count("src/schema.rs", source), 1, "{source}");
    }
}

#[test]
fn structural_assembly_uses_the_model_catalog() {
    let source = r#"
        pub struct Incoming { pub id: String }
        pub struct Action { pub id: String }
        fn convert(value: Incoming) -> Action { Action { id: value.id } }
    "#;
    assert_eq!(count("src/handler.rs", source), 1);
    assert_eq!(count("src/mappers/commands.rs", source), 0);
    let nested = r#"
        mod schemas { pub struct Wire { pub id: String } }
        use schemas::Wire as Envelope;
        fn handler(value: String) -> Envelope { Envelope { id: value } }
    "#;
    assert_eq!(count("src/handler.rs", nested), 1);
}

#[test]
fn aliases_and_closures_cannot_hide_model_assembly() {
    let source = r#"
        pub struct Incoming { pub id: String }
        pub struct Action { pub id: String }
        type Output = Action;
        fn handler(values: Vec<Incoming>) -> Vec<Output> {
            values.into_iter().map(|value| {
                let convert = || Output { id: value.id };
                convert()
            }).collect()
        }
    "#;
    assert_eq!(count("src/handler.rs", source), 1);
    let self_output = r#"
        pub struct Incoming { pub id: String }
        pub struct Action { pub id: String }
        impl Action { fn convert(value: Incoming) -> Self { Self { id: value.id } } }
    "#;
    assert_eq!(count("src/handler.rs", self_output), 1);
}

#[test]
fn macro_payloads_and_templates_are_checked() {
    for body in
        ["vec![Wire { id: value }]", "params![Wire { id: value }]", "vec![Wire { id: value }; 3]"]
    {
        let source = format!(
            "mod schemas {{ pub struct Wire {{ pub id: String }} }} use schemas::Wire; fn \
             handler(value: String) {{ let _ = {body}; }}"
        );
        assert_eq!(count("src/handler.rs", &source), 1, "{body}");
    }
    let implementation = r#"
        macro_rules! convert {
            ($name:ident) => { impl From<Input> for $name { fn from(value: Input) -> Self { todo!() } } };
        }
    "#;
    assert_eq!(count("src/legacy.rs", implementation), 1);
    let enum_conversion = r#"
        macro_rules! string_enum {
            ($name:ident { $($variant:ident => $value:literal),+ }) => {
                pub enum $name { $($variant),+ }
                impl $name { fn as_str(self) -> &'static str { match self { $(Self::$variant => $value),+ } } }
            };
        }
    "#;
    assert_eq!(count("src/legacy.rs", enum_conversion), 1);
}

#[test]
fn snapshot_and_collection_projections_are_checked() {
    let source = r#"
        pub struct Snapshot { pub id: String }
        pub struct Entity { state: Snapshot }
        impl Entity { fn snapshot(&self) -> Snapshot { self.state.clone() } }
    "#;
    assert_eq!(count("src/entity.rs", source), 1);
    assert_eq!(
        count(
            "src/adapter.rs",
            "fn values(input: Vec<Row>) { input.iter().map(|row| (row.id.to_string(), \
             row.name.clone())).collect::<Vec<_>>(); }"
        ),
        1
    );
    assert_eq!(
        count(
            "src/adapter.rs",
            "fn values(input: Vec<Row>) { input.iter().map(|row| (row.id, \
             row.name)).collect::<Vec<_>>(); }"
        ),
        1
    );
    assert_eq!(
        count(
            "src/adapter.rs",
            "use hiqlite::params as arguments; fn write(row: Row) { arguments![row.id, row.name]; \
             }"
        ),
        1
    );
}

#[test]
fn standard_mapper_delegation_and_business_algorithms_are_legal() {
    let source = r#"
        pub struct Snapshot { pub id: String }
        pub struct Entity { state: Snapshot }
        impl Entity {
            fn restore(state: Snapshot) -> Result<Self, Error> {
                validate(&state)?;
                Ok(Self { state })
            }
            fn snapshot(&self) -> Snapshot { crate::mappers::entity::snapshot(self) }
        }
        fn execute(values: Vec<Input>) -> Vec<Output> { values.into_iter().map(Into::into).collect() }
        fn lookup(values: Vec<Input>) { let indexed = values.into_iter().map(|v| (v.id, v)).collect::<Map<_, _>>(); }
        fn validate(value: &Snapshot) -> Result<(), Error> {
            if value.id.is_empty() { return Err(Error::Invalid("missing id".into())); }
            Ok(())
        }
    "#;
    assert_eq!(count("src/entity.rs", source), 0);
    let business = r#"
        pub struct Amount { raw: String, scaled: i128 }
        impl Amount {
            fn parse(raw: &str) -> Result<Self, Error> {
                let scaled = raw.parse::<i128>().map_err(|_| Error::Invalid)?;
                if scaled < 0 { return Err(Error::Invalid); }
                Ok(Self { raw: raw.to_owned(), scaled })
            }
        }
    "#;
    assert_eq!(count("apps/domain/src/values/amount.rs", business), 0);
}

#[test]
fn service_wiring_and_framework_wrappers_are_legal() {
    let source = r#"
        pub struct Handler { port: Arc<dyn Port> }
        pub enum Failure { Invalid }
        impl Handler { fn from_dependencies(port: Arc<dyn Port>) -> Result<Self, Failure> { Ok(Self { port }) } }
        pub struct State { pub handler: Handler }
        fn wire(port: Arc<dyn Port>) -> State { State { handler: Handler { port } } }
        fn request(value: Request) { value.map(Body::new); }
        fn response(value: Value) { Json(value).into_response(); }
    "#;
    assert_eq!(count("src/di.rs", source), 0);
}

#[test]
fn only_guaranteed_test_cfg_scopes_are_ignored() {
    let implementation =
        "impl From<Input> for Output { fn from(value: Input) -> Self { todo!() } }";
    for attribute in [
        "#[cfg(test)]",
        "#[cfg(all(test, feature = \"extra\"))]",
        "#[cfg(any(test, all(test, unix))) ]",
    ] {
        assert_eq!(
            count(
                "src/production.rs",
                &format!("{attribute} mod unit {{ {implementation} }}")
            ),
            0,
            "{attribute}"
        );
    }
    for attribute in [
        "#[cfg(any(test, feature = \"extra\"))]",
        "#[cfg(not(test))]",
        "#[cfg_attr(test, allow(dead_code))]",
    ] {
        assert_eq!(
            count(
                "src/production_tests.rs",
                &format!("{attribute} mod unit {{ {implementation} }}")
            ),
            1,
            "{attribute}"
        );
    }
    assert_eq!(
        count(
            "src/testing.rs",
            &format!("mod tests {{ {implementation} }}")
        ),
        1
    );
}

#[test]
fn test_file_classification_follows_module_reachability() {
    let sources: Vec<_> = [
        (
            "src/lib.rs",
            "#[cfg(test)] mod tests; #[cfg(any(test, feature = \"production\"))] mod shared_tests;",
        ),
        ("src/tests.rs", "mod fixtures;"),
        ("src/tests/fixtures.rs", ""),
        ("src/shared_tests.rs", ""),
        ("tests/integration.rs", ""),
    ]
    .into_iter()
    .map(|(path, source)| {
        Source {
            path: path.into(),
            scope: source_scope(path.as_ref()),
            syntax: syn::parse_file(source).unwrap(),
        }
    })
    .collect();
    let actual = test_files(&sources);
    assert!(actual.contains(std::path::Path::new("src/tests.rs")));
    assert!(actual.contains(std::path::Path::new("src/tests/fixtures.rs")));
    assert!(actual.contains(std::path::Path::new("tests/integration.rs")));
    assert!(!actual.contains(std::path::Path::new("src/shared_tests.rs")));
}

#[test]
fn the_git_scoped_checker_exercises_actual_rust_sources_without_rewriting() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    fs::create_dir_all(root.join("src/mappers")).unwrap();
    fs::create_dir_all(root.join("target/generated")).unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    let source = "impl From<Input> for Output { fn from(value: Input) -> Self { todo!() } }";
    for path in [
        "src/mappers/conversion.rs",
        "src/mappers.rs",
        "src/testing.rs",
        "target/generated/bindings.rs",
    ] {
        fs::write(root.join(path), source).unwrap();
    }
    fs::write(root.join("src/lib.rs"), "#[cfg(test)] mod testing;").unwrap();
    assert_eq!(check_mappings(root).unwrap(), 1);
    assert_eq!(
        fs::read_to_string(root.join("src/mappers.rs")).unwrap(),
        source
    );
    fs::write(root.join("src/invalid.rs"), "fn invalid(").unwrap();
    assert!(check_mappings(root).unwrap_err().to_string().contains("src/invalid.rs"));
}

#[cfg(unix)]
#[test]
fn a_symlink_into_mappers_does_not_hide_the_actual_location() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    fs::create_dir_all(root.join("src/mappers")).unwrap();
    fs::write(
        root.join("src/legacy.rs"),
        "impl From<Input> for Output { fn from(value: Input) -> Self { todo!() } }",
    )
    .unwrap();
    std::os::unix::fs::symlink("../legacy.rs", root.join("src/mappers/alias.rs")).unwrap();
    assert_eq!(check_mappings(root).unwrap(), 2);
}

#[test]
fn qualified_reexports_preserve_the_defining_package() {
    let sources: Vec<_> = [
        (
            "apps/application/src/commands.rs",
            "pub struct Action { pub id: String }",
        ),
        (
            "apps/application/src/lib.rs",
            "mod commands; pub use commands::Action;",
        ),
        (
            "src/api/handlers/example.rs",
            "use application::Action as Input; fn handler(value: String) -> Input { Input { id: \
             value } }",
        ),
    ]
    .into_iter()
    .map(|(path, source)| {
        Source {
            path: path.into(),
            scope: source_scope(path.as_ref()),
            syntax: syn::parse_file(source).unwrap(),
        }
    })
    .collect();
    let mut catalog = Catalog::default();
    for source in &sources {
        catalog.collect(&source.scope, &source.syntax.items);
    }
    assert_eq!(violations(&sources[2], &catalog).len(), 1);
}

#[test]
fn a_same_named_macro_import_does_not_shadow_the_external_crate() {
    let source = Source {
        path: "src/config.rs".into(),
        scope: source_scope("src/config.rs".as_ref()),
        syntax: syn::parse_file("use anyhow::{Result, anyhow};").unwrap(),
    };
    let mut catalog = Catalog::default();
    catalog.collect(&source.scope, &source.syntax.items);
    let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        catalog.resolve(&source.scope, &names(&["Result"])),
        names(&["anyhow", "Result"])
    );
    assert_eq!(
        catalog.resolve(&source.scope, &names(&["anyhow"])),
        names(&["anyhow", "anyhow"])
    );
    assert_eq!(
        catalog.resolve(&source.scope, &names(&["anyhow", "Result"])),
        names(&["anyhow", "Result"])
    );
    assert_eq!(
        count("src/config.rs", include_str!("../../../../src/config.rs")),
        0
    );
}

#[test]
fn cyclic_aliases_terminate_without_a_recursion_depth_limit() {
    let source = Source {
        path: "src/conversions.rs".into(),
        scope: source_scope("src/conversions.rs".as_ref()),
        syntax: syn::parse_file("type First = Second; type Second = Third; type Third = First;")
            .unwrap(),
    };
    let mut catalog = Catalog::default();
    catalog.collect(&source.scope, &source.syntax.items);
    assert!(!catalog.resolve(&source.scope, &["First".into()]).is_empty());
}

#[test]
fn mapper_namespace_files_cannot_hide_untyped_or_inherent_implementations() {
    for source in [
        "fn convert(value: i32) -> i64 { value as i64 }",
        "impl Wire { fn convert(self) -> Other { other(self) } }",
        "mod nested { fn convert(value: i32) -> i64 { value as i64 } }",
    ] {
        assert_eq!(count("src/mappers.rs", source), 1, "{source}");
    }
    assert_eq!(
        count(
            "src/mappers.rs",
            "mod nested { pub mod crf; pub use crf::*; }"
        ),
        0
    );
    assert_eq!(
        count(
            "src/mappers.rs",
            "#[cfg(test)] fn fixture() {} #[cfg(test)] type Fixture = ();"
        ),
        0
    );
}

#[test]
fn private_data_projections_require_mappers_in_free_and_inherent_functions() {
    for conversion in [
        "fn convert(value: Value) -> Stored { Stored { id: value.id } }",
        "impl Stored { fn convert(value: Value) -> Self { Self { id: value.id } } }",
        "fn convert(value: Value) -> Stored { let id = value.id; Stored { id } }",
        "fn convert(Value { id }: Value) -> Stored { Stored { id } }",
    ] {
        let source =
            format!("struct Stored {{ id: String }} struct Value {{ id: String }} {conversion}");
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{conversion}"
        );
        assert_eq!(
            count("apps/infrastructure/src/mappers/values.rs", &source),
            0
        );
    }
    let validated = r#"
        struct State { id: String }
        struct Entity { state: State }
        impl Entity {
            fn restore(state: State) -> Result<Self, Error> {
                if state.id.is_empty() { return Err(Error::Invalid); }
                validate(&state)?;
                Ok(Self { state })
            }
        }
    "#;
    assert_eq!(count("apps/domain/src/entities/entity.rs", validated), 0);
}

#[test]
fn known_function_results_and_local_field_aliases_preserve_projection_types() {
    for functions in [
        "fn source() -> Input { todo!() } fn convert() -> Output { let value = source(); Output { \
         id: value.id } }",
        "fn source() -> Result<Input, Error> { todo!() } fn convert() -> Result<Output, Error> { \
         let value = source()?; Ok(Output { id: value.id }) }",
        "async fn source() -> Result<Input, Error> { todo!() } async fn convert() -> \
         Result<Output, Error> { let value = source().await?; let alias = value; let id = \
         alias.id; Ok(Output { id }) }",
        "fn source() -> Input { todo!() } fn convert() -> Output { let id = source().id; let \
         alias = id; Output { id: alias } }",
        "impl Input { fn source() -> Result<Input, Error> { todo!() } } fn convert() -> \
         Result<Output, Error> { let value = Input::source()?; Ok(Output { id: value.id }) }",
        "struct Envelope { value: Input } fn source() -> Envelope { todo!() } fn convert() -> \
         Output { let envelope = source(); let value = envelope.value; Output { id: value.id } }",
    ] {
        let source = format!(
            "pub struct Input {{ pub id: String }} pub struct Output {{ pub id: String }} \
             {functions}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{functions}"
        );
    }
}

#[test]
fn block_and_parent_glob_conversion_aliases_are_resolved_without_leaking() {
    for source in [
        "fn register() { use std::convert::From as Map; impl Map<Input> for Output { fn \
         from(value: Input) -> Self { todo!() } } }",
        "fn register() { use core::convert as conv; use conv::TryFrom as Map; impl Map<Input> for \
         Output { type Error = (); fn try_from(value: Input) -> Result<Self, ()> { todo!() } } }",
        "mod outer { use core::convert::From as Map; mod inner { use super::*; impl Map<Input> \
         for Output { fn from(value: Input) -> Self { todo!() } } } }",
        "fn register() { use derive_more::From as Map; #[derive(Map)] struct Output(Input); }",
    ] {
        assert!(
            count("apps/infrastructure/src/adapter.rs", source) > 0,
            "{source}"
        );
    }
    let separate = "trait Map<T> {} fn register() { { use std::convert::From as Map; } impl \
                    Map<Input> for Output {} }";
    assert_eq!(count("apps/infrastructure/src/adapter.rs", separate), 0);
}

#[test]
fn pure_scalar_sequence_projections_include_local_intermediates() {
    for body in [
        "input.into_iter().map(|value| value.id).collect()",
        "input.into_iter().map(|value| { value.id }).collect()",
        "let result = input.into_iter().map(|value| value.id).collect(); result",
        "input.into_iter().map(|value| { let id = value.id; id }).collect()",
        "let values = input; return values.into_iter().map(|value| value.id.clone()).collect();",
    ] {
        let source = format!(
            "pub struct Input {{ pub id: String }} fn convert(input: Vec<Input>) -> Vec<String> \
             {{ {body} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{body}"
        );
    }
}

#[test]
fn literal_property_collections_require_mappers_for_known_field_origins() {
    let borrowed = r#"
        struct Text { name: Option<String>, description: Option<String> }
        impl Text {
            fn properties(&self) -> Vec<(&'static str, Option<&str>)> {
                vec![("name", self.name.as_deref()), ("description", self.description.as_deref())]
            }
        }
    "#;
    assert!(count("apps/domain/src/localization.rs", borrowed) > 0);
    for body in [
        "vec![(\"name\", value.name)]",
        "std::vec![(\"name\", value.name)]",
        "alloc::vec![(\"name\", value.name)]",
        "use std::vec as sequence; sequence![(\"name\", value.name)]",
        "let name = value.name; let properties = vec![(\"name\", name)]; properties",
        "let property = (\"name\", value.name); vec![property]",
        "[(\"name\", value.name)].to_vec()",
        "Vec::from([(\"name\", value.name)])",
        "[(\"name\", value.name)].into_iter().collect()",
        "vec![(\"name\", value.name); 2]",
    ] {
        let source = format!(
            "struct Text {{ name: String }} type Properties<T> = Vec<(&'static str, T)>; fn \
             extract(value: Text) -> Properties<String> {{ {body} }}"
        );
        assert!(
            count("apps/domain/src/localization.rs", &source) > 0,
            "{body}"
        );
        assert_eq!(
            count("apps/domain/src/mappers/localization.rs", &source),
            0,
            "{body}"
        );
    }
    let array = "struct Text { name: String } fn extract(value: Text) -> [(&'static str, String); \
                 1] { [(\"name\", value.name)] }";
    assert!(count("apps/domain/src/localization.rs", array) > 0);
}

#[test]
fn literal_collections_keep_business_work_and_scalar_getters_legal() {
    for source in [
        "struct Text { name: Option<String> } impl Text { fn name(&self) -> Option<&str> { \
         self.name.as_deref() } }",
        "struct Text { name: String } fn index(value: Text) -> BTreeMap<&'static str, String> { \
         BTreeMap::from([(\"name\", value.name)]) }",
        "struct Input { amount: i64 } fn calculate(value: Input) -> Vec<(&'static str, i64)> { \
         vec![(\"amount\", value.amount + 1)] }",
        "struct Input { amount: i64 } fn calculate(value: Input) -> Vec<(&'static str, i64)> { \
         let amount = value.amount + 1; vec![(\"amount\", amount)] }",
        "struct Input { amounts: Vec<i64> } fn select(value: Input, index: usize) -> \
         Vec<(&'static str, i64)> { vec![(\"amount\", value.amounts[index])] }",
        "struct Input { amount: i64 } fn validate(value: Input) -> Vec<(&'static str, i64)> { if \
         value.amount <= 0 { return Vec::new(); } vec![(\"amount\", value.amount)] }",
        "struct Input { amount: i64 } fn validate(value: Input) -> Vec<(&'static str, i64)> { let \
         valid = validate_amount!(value.amount); vec![(\"amount\", value.amount)] }",
        "fn constants() -> Vec<(&'static str, &'static str)> { vec![(\"name\", \"value\")] }",
        "fn scalar(name: String) -> Vec<(&'static str, String)> { vec![(\"name\", name)] }",
    ] {
        assert_eq!(count("apps/domain/src/authoring.rs", source), 0, "{source}");
    }
}

#[test]
fn transparent_blocks_preserve_collection_tuple_and_scalar_map_projections() {
    for body in [
        "{ vec![(\"name\", value.name)] }",
        "({ vec![(\"name\", value.name)] })",
        "{ let properties = vec![(\"name\", value.name)]; properties }",
        "{ let name = value.name; vec![(\"name\", name)] }",
        "let result = { vec![(\"name\", value.name)] }; result",
        "let result = { let properties = vec![(\"name\", value.name)]; properties }; result",
        "return { let properties = vec![(\"name\", value.name)]; properties };",
        "{ use std::vec as properties; properties![(\"name\", value.name)] }",
        "Vec::from({ [(\"name\", value.name)] })",
    ] {
        let source = format!(
            "struct Text {{ name: String }} fn properties(value: Text) -> Vec<(&'static str, \
             String)> {{ {body} }}"
        );
        assert!(
            count("apps/domain/src/localization.rs", &source) > 0,
            "{body}"
        );
    }
    for body in [
        "{ (value.name, value.description) }",
        "let result = { let name = value.name; (name, value.description) }; result",
        "return { let result = (value.name, value.description); result };",
    ] {
        let source = format!(
            "struct Text {{ name: String, description: String }} fn properties(value: Text) -> \
             (String, String) {{ {body} }}"
        );
        assert!(
            count("apps/domain/src/localization.rs", &source) > 0,
            "{body}"
        );
    }
    for body in [
        "input.into_iter().map(|value| { { value.name } }).collect()",
        "input.into_iter().map(|value| { let name = { value.name }; name }).collect()",
    ] {
        let source = format!(
            "struct Text {{ name: String }} fn properties(input: Vec<Text>) -> Vec<String> {{ \
             {body} }}"
        );
        assert!(
            count("apps/domain/src/localization.rs", &source) > 0,
            "{body}"
        );
    }
}

#[test]
fn block_bindings_capture_origins_and_types_before_later_shadows() {
    for body in [
        "let name = value.name; let result = vec![(\"name\", name)]; let name = plain; result",
        "let result = { let name = value.name; vec![(\"name\", name)] }; let name = plain; result",
        "let name = value.name; let ignored = { let name = plain; name }; vec![(\"name\", name)]",
        "let saved = { let alias = value; alias }; let value = plain; vec![(\"name\", saved.name)]",
        "let fields = { let name = value.name; (name, plain) }; let value = 0; let (name, _) = \
         fields; vec![(\"name\", name)]",
        "{ let name = value.name; let name = name.clone(); vec![(\"name\", name)] }",
    ] {
        let source = format!(
            "struct Text {{ name: String }} fn properties(value: Text, plain: String) -> \
             Vec<(&'static str, String)> {{ {body} }}"
        );
        assert!(
            count("apps/domain/src/localization.rs", &source) > 0,
            "{body}"
        );
    }
    for body in [
        "let result = vec![(\"name\", name)]; let name = value.name; result",
        "let result = { let alias = name; vec![(\"name\", alias)] }; let name = value.name; result",
        "let ignored = { let name = value.name; name }; vec![(\"name\", name)]",
        "let name = value.name; let name = plain; vec![(\"name\", name)]",
        "let fields = (name, plain); let name = value.name; let (name, _) = fields; \
         vec![(\"name\", name)]",
        "{ use opaque::vec as properties; properties![(\"name\", value.name)] }",
        "let ignored = { use std::vec as properties; 0 }; properties![(\"name\", value.name)]",
    ] {
        let source = format!(
            "struct Text {{ name: String }} fn properties(value: Text, name: String, plain: \
             String) -> Vec<(&'static str, String)> {{ {body} }}"
        );
        assert_eq!(count("apps/domain/src/authoring.rs", &source), 0, "{body}");
    }
}

#[test]
fn transparent_blocks_keep_guards_calculations_indexing_and_getters_legal() {
    for source in [
        "struct Input { amount: i64 } fn calculate(value: Input) -> Vec<(&'static str, i64)> { { \
         let amount = value.amount + 1; vec![(\"amount\", amount)] } }",
        "struct Input { amount: i64 } fn calculate(value: Input) -> (i64,) { { let amount = \
         value.amount + 1; (amount,) } }",
        "struct Input { amounts: Vec<i64> } fn select(value: Input, index: usize) -> Vec<i64> { { \
         vec![value.amounts[index]] } }",
        "struct Input { amount: i64 } fn validate(value: Input) -> Vec<i64> { { if value.amount \
         <= 0 { return Vec::new(); } vec![value.amount] } }",
        "struct Input { amount: i64 } fn validate(value: Input) -> Vec<i64> { { let valid = \
         validate_amount!(value.amount); vec![value.amount] } }",
        "struct Text { name: String } impl Text { fn name(&self) -> &str { { let name = \
         self.name.as_str(); name } } }",
        "struct Input { amount: i64 } fn calculate(input: Vec<Input>) -> Vec<i64> { \
         input.into_iter().map(|value| { { value.amount + 1 } }).collect() }",
        "struct Input { amount: i64 } fn select(input: Vec<Input>) -> Vec<i64> { \
         input.into_iter().map(|value| { if value.amount <= 0 { return 0; } value.amount \
         }).collect() }",
    ] {
        assert_eq!(count("apps/domain/src/authoring.rs", source), 0, "{source}");
    }
}

#[test]
fn business_selection_calculation_and_runtime_dependency_containers_remain_legal() {
    let business = r#"
        struct Input { id: String, amount: i64 }
        fn enabled(input: Vec<Input>) -> Vec<String> {
            input.into_iter().filter(|value| value.amount > 0).map(|value| value.id).collect()
        }
        fn calculate(input: Vec<Input>) -> Vec<i64> { input.into_iter().map(|value| value.amount + 1).collect() }
        fn calculated(input: Vec<Input>) -> Vec<i64> { input.into_iter().map(|value| calculate_amount(value.amount)).collect() }
        fn index(input: Vec<Input>) -> Set<String> { input.into_iter().map(|value| value.id).collect() }
        fn choose(head: Head) -> Option<String> { head.latest_frozen.map(|value| value.id) }
        fn validate(input: Vec<Input>) -> Vec<String> {
            if input.is_empty() { panic!(); }
            input.into_iter().map(|value| value.id).collect()
        }
        fn lookup(names: Vec<String>, fields: Option<Map>) -> Option<(Vec<String>, Map)> {
            fields.map(|fields| (names[..names.len() - 1].to_vec(), fields.clone()))
        }
    "#;
    assert_eq!(count("apps/domain/src/authoring.rs", business), 0);
    let runtime = r#"
        pub struct Service { port: Arc<dyn Port> }
        pub struct Container { pub service: Option<Service> }
        pub struct Wiring { pub ports: Vec<Arc<dyn Port>>, pub services: [Service; 1], pub pair: (Service, usize) }
        fn wire(input: Wiring, service: Service) -> Container { Container { service: Some(service) } }
        fn copy_wiring(input: Wiring) -> Wiring { Wiring { ports: input.ports, services: input.services, pair: input.pair } }
        struct Catalog { entries: Vec<String> }
        struct Source { scope: Vec<String> }
        struct Visitor<'a> { catalog: &'a Catalog, scope: Vec<String>, pending: Vec<String> }
        impl<'a> Visitor<'a> {
            fn new(catalog: &'a Catalog, scope: Vec<String>) -> Self { Self { catalog, scope, pending: Vec::new() } }
        }
        fn initialize<'a>(catalog: &'a Catalog, source: Source) -> Visitor<'a> { Visitor::new(catalog, source.scope.clone()) }
        type Ports<T: ?Sized> = Vec<Arc<T>>;
        struct Settings { name: String }
        struct Runtime { name: String, ports: Ports<dyn Port> }
        fn runtime(settings: Settings, ports: Ports<dyn Port>) -> Runtime { Runtime { name: settings.name, ports } }
    "#;
    assert_eq!(count("src/di.rs", runtime), 0);
}

#[test]
fn tuple_values_and_patterns_preserve_each_field_origin() {
    for body in [
        "let (id,) = (input.id,); Output { id }",
        "let fields = (input.id,); let (id,) = fields; Output { id }",
        "let (id, _) = (input.id, input.amount + 1); Output { id }",
    ] {
        let source = format!(
            "struct Input {{ id: String, amount: i64 }} struct Output {{ id: String }} fn \
             convert(input: Input) -> Output {{ {body} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{body}"
        );
    }
    let direct = "struct Input { id: String, label: String } fn convert(input: Input) -> (String, \
                  String) { (input.id, input.label) }";
    assert!(count("apps/infrastructure/src/adapter.rs", direct) > 0);
    let calculated = "struct Input { id: String, amount: i64 } struct Output { amount: i64 } fn \
                      calculate(input: Input) -> Output { let (_, amount) = (input.id, \
                      input.amount + 1); Output { amount } }";
    assert_eq!(count("apps/domain/src/authoring.rs", calculated), 0);
}

#[test]
fn tuple_rest_patterns_keep_the_matching_type_and_field_origin() {
    for body in [
        "let (.., id) = (0, 0, input.id); Output { id }",
        "let (_, .., id) = (0, 0, input.id); Output { id }",
        "let (id, ..) = (input.id, 0, 0); Output { id }",
        "let fields = (0, 0, input.id); let (.., id) = fields; Output { id }",
        "let (.., (id, ..)) = (0, (input.id, 0)); Output { id }",
    ] {
        let source = format!(
            "struct Input {{ id: String }} struct Output {{ id: String }} fn convert(input: \
             Input) -> Output {{ {body} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{body}"
        );
    }
    for pattern in ["(.., value)", "(_, .., value)"] {
        let source = format!(
            "struct Input {{ id: String }} struct Output {{ id: String }} fn convert({pattern}: \
             (usize, usize, Input)) -> Output {{ Output {{ id: value.id }} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{pattern}"
        );
    }
    for body in [
        "let (.., amount) = (0, input.id, input.amount + 1); Output { amount }",
        "let (_, .., amount) = (0, 0, input.id, input.amount + 1); Output { amount }",
    ] {
        let source = format!(
            "struct Input {{ id: String, amount: i64 }} struct Output {{ amount: i64 }} fn \
             calculate(input: Input) -> Output {{ {body} }}"
        );
        assert_eq!(count("apps/domain/src/authoring.rs", &source), 0, "{body}");
    }
}

#[test]
fn tuple_struct_rest_patterns_bind_the_last_declared_field_type() {
    for declaration in
        ["struct Record(usize, usize, Input);", "enum Record { Value(usize, usize, Input) }"]
    {
        let source = Source {
            path: "apps/infrastructure/src/adapter.rs".into(),
            scope: source_scope("apps/infrastructure/src/adapter.rs".as_ref()),
            syntax: syn::parse_file(&format!("struct Input {{ id: String }} {declaration}"))
                .unwrap(),
        };
        let mut catalog = Catalog::default();
        catalog.collect(&source.scope, &source.syntax.items);
        let mut inspector = super::Inspector::new(&catalog, source.scope.clone());
        let pattern: syn::Pat = if declaration.starts_with("enum") {
            syn::parse_quote!(Record::Value(.., value))
        }
        else {
            syn::parse_quote!(Record(.., value))
        };
        inspector.bind(&pattern, syn::parse_quote!(Record));
        let path = super::type_path(inspector.variables.get("value").unwrap()).unwrap();
        assert_eq!(
            path.segments.last().unwrap().ident,
            "Input",
            "{declaration}"
        );
    }
    for wrapper in ["Some", "Ok", "Err"] {
        let ty = if wrapper == "Some" {
            syn::parse_quote!(Option<Input>)
        }
        else {
            syn::parse_quote!(Result<Input, Input>)
        };
        let wrapper = syn::Ident::new(wrapper, proc_macro2::Span::call_site());
        let pattern = syn::parse_quote!(#wrapper(.., value));
        let catalog = Catalog::default();
        let mut inspector = super::Inspector::new(&catalog, Vec::new());
        inspector.bind(&pattern, ty);
        assert!(
            super::type_path(inspector.variables.get("value").unwrap()).unwrap().is_ident("Input")
        );
    }
}

#[test]
fn loops_match_arms_and_closures_bind_declared_fields() {
    for source in [
        "struct Input { id: String } struct Output { id: String } fn convert(input: Vec<Input>) \
         -> Vec<Output> { let mut values = Vec::new(); for value in input { values.push(Output { \
         id: value.id }); } values }",
        "struct Input { id: String } struct Output { id: String } fn convert(input: Vec<Input>) \
         -> Vec<Output> { let mut values = Vec::new(); for Input { id } in input { \
         values.push(Output { id }); } values }",
        "enum Input { Id(String) } struct Output { id: String } fn convert(input: Input) -> \
         Output { match input { Input::Id(id) => Output { id } } }",
        "enum Input { Value { id: String } } struct Output { id: String } fn convert(input: \
         Input) -> Output { match input { Input::Value { id } => Output { id } } }",
        "struct Input(String); struct Output { id: String } fn convert(input: Input) -> Output { \
         let Input(id) = input; Output { id } }",
        "struct Input { id: String } struct Output { id: String } fn convert(input: Vec<Input>) \
         -> Vec<Output> { input.into_iter().map(|Input { id }| Output { id }).collect() }",
        "struct Input { id: String } struct Output { id: String } fn convert(input: \
         Vec<(Input,)>) -> Vec<Output> { input.into_iter().map(|(Input { id },)| Output { id \
         }).collect() }",
    ] {
        assert!(
            count("apps/infrastructure/src/adapter.rs", source) > 0,
            "{source}"
        );
    }
}

#[test]
fn local_enum_representations_and_tuple_models_require_mappers() {
    for source in [
        "struct Input { id: String } enum Output { Id(String) } fn convert(input: Input) -> \
         Output { Output::Id(input.id) }",
        "enum Input { A, B } enum Output { A, B } fn convert(input: Input) -> Output { match \
         input { Input::A => Output::A, Input::B => Output::B } }",
        "struct Input { id: String } struct Output(String); fn convert(input: Input) -> Output { \
         Output(input.id) }",
    ] {
        assert!(
            count("apps/infrastructure/src/adapter.rs", source) > 0,
            "{source}"
        );
    }
    for source in [
        "enum Input { Value(i64) } enum Output { Positive, NonPositive } fn decide(input: Input) \
         -> Output { match input { Input::Value(value) if value > 0 => Output::Positive, _ => \
         Output::NonPositive } }",
        "enum State { Draft, Published } fn transition(value: State) -> State { match value { \
         State::Draft => State::Published, State::Published => State::Published } }",
        "struct Input { id: String } struct Output { id: String } impl Output { fn new(id: \
         String) -> Self { Self { id } } } fn initialize(input: Input) -> Output { \
         Output::new(input.id) }",
        "struct Entity { id: String } impl Entity { fn id(&self) -> &str { self.id.as_str() } }",
    ] {
        assert_eq!(count("apps/domain/src/authoring.rs", source), 0, "{source}");
    }
}

#[test]
fn condition_payloads_and_let_chains_bind_without_leaking() {
    for body in [
        "if let Input::Id(id) = input { Some(Output { id }) } else { None }",
        "if let Input::Id(id) = input && !id.is_empty() { Some(Output { id }) } else { None }",
        "while let Input::Id(id) = input { return Some(Output { id }); } None",
    ] {
        let source = format!(
            "enum Input {{ Id(String), Empty }} struct Output {{ id: String }} fn convert(input: \
             Input) -> Option<Output> {{ {body} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{body}"
        );
    }
    for argument in ["Option<Input>", "Result<Input, Error>"] {
        let wrapper = if argument.starts_with("Option") { "Some" } else { "Ok" };
        let source = format!(
            "struct Input {{ id: String }} struct Output {{ id: String }} fn convert(input: \
             {argument}) -> Option<Output> {{ if let {wrapper}(Input {{ id }}) = input {{ \
             Some(Output {{ id }}) }} else {{ None }} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{argument}"
        );
    }
    let scoped = r#"
        enum Input { Id(String), Empty }
        struct Record { id: String }
        struct Output { id: String }
        fn select(input: Input, values: Vec<Record>, id: String) -> Output {
            for Record { id } in values { drop(id); }
            match input { Input::Id(id) => drop(id), _ => () }
            if let Input::Id(id) = Input::Empty { drop(id); } else { drop(&id); }
            while let Input::Id(id) = Input::Empty { drop(id); }
            Output { id }
        }
    "#;
    assert_eq!(count("apps/domain/src/authoring.rs", scoped), 0);
}

#[test]
fn self_and_local_aliases_expand_recursively_with_type_arguments() {
    let inherent = "struct Input { id: String } struct Output { id: String } impl Input { fn \
                    source() -> Result<Self, ()> { todo!() } } fn convert() -> Result<Output, ()> \
                    { let value = Input::source()?; Ok(Output { id: value.id }) }";
    assert!(count("apps/infrastructure/src/adapter.rs", inherent) > 0);
    for aliases in [
        "type Names = Vec<String>;",
        "type Sequence<T> = Vec<T>; type Names = Sequence<String>;",
        "mod values { pub type Names = Vec<String>; } use values::Names;",
    ] {
        let source = format!(
            "struct Input {{ id: String }} {aliases} fn convert(input: Vec<Input>) -> Names {{ \
             input.into_iter().map(|value| value.id).collect() }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{aliases}"
        );
    }
    let generic_input = "struct Input { id: String } struct Output { id: String } type Items<T> = \
                         Vec<T>; fn convert(input: Items<Input>) -> Vec<Output> { let mut result \
                         = Vec::new(); for value in input { result.push(Output { id: value.id }); \
                         } result }";
    assert!(count("apps/infrastructure/src/adapter.rs", generic_input) > 0);
    assert_eq!(
        count(
            "src/runtime.rs",
            "type First = Second; type Second = First; fn unavailable() -> First { todo!() }"
        ),
        0
    );
}

#[test]
fn from_iterator_implementations_follow_the_conversion_trait_rule() {
    for declaration in [
        "impl std::iter::FromIterator<Input> for Output { fn from_iter<T: IntoIterator<Item = \
         Input>>(values: T) -> Self { todo!() } }",
        "use core::iter::FromIterator as Gather; impl Gather<Input> for Output { fn from_iter<T: \
         IntoIterator<Item = Input>>(values: T) -> Self { todo!() } }",
    ] {
        assert_eq!(count("src/collection.rs", declaration), 1);
        assert_eq!(count("src/mappers/collection.rs", declaration), 0);
    }
}

#[test]
fn map_iterators_bind_both_keys_and_values_in_loops_and_closures() {
    for map in ["BTreeMap", "HashMap"] {
        for body in [
            "input.into_iter().map(|(_, value)| Output { id: value.id }).collect()",
            "let mut result = Vec::new(); for (_, value) in input { result.push(Output { id: \
             value.id }); } result",
            "let mut result = Vec::new(); for (_, Input { id }) in input { result.push(Output { \
             id }); } result",
        ] {
            let source = format!(
                "struct Input {{ id: String }} struct Output {{ id: String }} fn convert(input: \
                 {map}<String, Input>) -> Vec<Output> {{ {body} }}"
            );
            assert!(
                count("apps/infrastructure/src/adapter.rs", &source) > 0,
                "{map}: {body}"
            );
        }
    }
    let index = "struct Input { id: String } fn index(input: Vec<Input>) -> BTreeMap<String, \
                 Input> { input.into_iter().map(|value| (value.id.clone(), value)).collect() }";
    assert_eq!(count("apps/domain/src/authoring.rs", index), 0);
    for body in [
        "input.into_iter().filter(|value| value.enabled).map(|value| Output { id: value.id \
         }).collect()",
        "input.into_iter().enumerate().map(|(_, value)| Output { id: value.id }).collect()",
        "let mut result = Vec::new(); for (_, Input { id, .. }) in input.into_iter().enumerate() \
         { result.push(Output { id }); } result",
    ] {
        let source = format!(
            "struct Input {{ id: String, enabled: bool }} struct Output {{ id: String }} fn \
             convert(input: Vec<Input>) -> Vec<Output> {{ {body} }}"
        );
        assert!(
            count("apps/infrastructure/src/adapter.rs", &source) > 0,
            "{body}"
        );
    }
    let filtered = "struct Input { id: String, enabled: bool } fn select(input: Vec<Input>) -> \
                    Vec<Input> { input.into_iter().filter(|value| value.enabled).collect() }";
    assert_eq!(count("apps/domain/src/authoring.rs", filtered), 0);
}

#[test]
fn map_key_and_value_iterators_keep_the_declared_element_type() {
    for map in ["BTreeMap", "HashMap"] {
        for method in ["values", "values_mut", "into_values", "keys", "into_keys"] {
            let types = if method.ends_with("keys") { "Input, String" } else { "String, Input" };
            for body in [
                format!(
                    "let mut result = Vec::new(); for value in input.{method}() {{ let output = \
                     Output {{ id: value.id.clone() }}; result.push(output); }} result"
                ),
                format!(
                    "input.{method}().map(|value| Output {{ id: value.id.clone() }}).collect()"
                ),
            ] {
                let source = format!(
                    "struct Input {{ id: String }} struct Output {{ id: String }} type Items<K, \
                     V> = {map}<K, V>; fn convert(mut input: Items<{types}>) -> Vec<Output> {{ \
                     {body} }}"
                );
                assert!(
                    count("apps/infrastructure/src/adapter.rs", &source) > 0,
                    "{map}::{method}: {body}"
                );
            }
            let source = format!(
                "struct Input {{ amount: i64 }} struct Output {{ amount: i64 }} fn calculate(mut \
                 input: {map}<{types}>) -> Vec<Output> {{ let mut result = Vec::new(); for value \
                 in input.{method}() {{ let output = Output {{ amount: value.amount + 1 }}; \
                 result.push(output); }} result }}"
            );
            assert_eq!(
                count("apps/domain/src/authoring.rs", &source),
                0,
                "{map}::{method}"
            );
        }
    }
}
