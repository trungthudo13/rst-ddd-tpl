use std::{
    fs,
    process::Command,
};

use super::{
    GRPC_MAPPER,
    HTTP_ADAPTER,
    SHARED_FILE,
    Source,
    Violation,
    check_errors,
    service_source,
    source_scope,
    violations,
};

const ENUM: &str = "pub enum Error { StudyDoesNotExist, Unexpected(String) }";
const HTTP_ACTION: &str = "use application::Error; pub struct ApiError(pub application::Error); \
                           impl axum::response::IntoResponse for ApiError { fn \
                           into_response(self) -> axum::response::Response { match self.0 { \
                           Error::StudyDoesNotExist => todo!(), Error::Unexpected(_) => todo!() } \
                           } }";
const GRPC_ACTION: &str = "fn map_error(error: application::Error) -> tonic::Status { match error \
                           { application::Error::StudyDoesNotExist => todo!(), \
                           application::Error::Unexpected(_) => todo!() } }";

fn sources(files: &[(&str, &str)]) -> Vec<Source> {
    let mut sources = Vec::new();
    for (path, source) in [
        (SHARED_FILE, ENUM),
        (
            "apps/shared/src/lib.rs",
            "pub mod errors; pub use errors::Error;",
        ),
        ("apps/application/src/lib.rs", "pub use shared::Error;"),
        ("apps/domain/src/lib.rs", "pub use shared::Error;"),
        (HTTP_ADAPTER, HTTP_ACTION),
    ]
    .into_iter()
    .filter(|(path, _)| !files.iter().any(|(given, _)| given == path))
    .chain(files.iter().copied())
    {
        let source = Source {
            path: path.to_owned(),
            scope: source_scope(path),
            syntax: syn::parse_file(source).unwrap(),
        };
        sources.push(source);
    }
    sources
}

fn inspect(files: &[(&str, &str)]) -> Vec<Violation> {
    violations(&sources(files))
}

fn reasons(source: &str) -> Vec<String> {
    inspect(&[("apps/domain/src/example.rs", source)])
        .into_iter()
        .map(|violation| violation.reason)
        .collect()
}

#[test]
fn accepts_shared_errors_through_direct_imports_reexports_and_aliases() {
    for source in [
        "fn run() -> Result<(), shared::Error> { todo!() }",
        "use crate::Error; fn run() -> Result<(), Error> { todo!() }",
        "use domain::Error as Failure; fn run() -> core::result::Result<(), Failure> { todo!() }",
        "use std::result::Result as Outcome; use shared::Error; fn run() -> Outcome<(), Error> { \
         todo!() }",
        "use shared::*; fn run() -> Result<(), Error> { todo!() }",
        "use shared::Error; type Outcome<T> = Result<T, Error>; fn run() -> Outcome<()> { todo!() \
         }",
        "mod nested { use super::super::Error as Failure; fn run() -> Result<(), Failure> { \
         todo!() } }",
        "use shared::Error; impl TryFrom<()> for Value { type Error = Error; fn try_from(_: ()) \
         -> Result<Self, Self::Error> { todo!() } }",
        "fn run() { use shared::Error as Failure; fn local() -> Result<(), Failure> { todo!() } }",
    ] {
        assert!(
            reasons(source).is_empty(),
            "{source}: {:?}",
            reasons(source)
        );
    }
}

#[test]
fn rejects_string_boxed_provider_and_generic_error_contracts() {
    for source in [
        "fn run() -> Result<(), String> { todo!() }",
        "fn run() -> std::result::Result<(), Box<dyn std::error::Error>> { todo!() }",
        "fn run() -> anyhow::Result<()> { todo!() }",
        "use anyhow::Result as Outcome; fn run() -> Outcome<()> { todo!() }",
        "fn run() -> std::io::Result<()> { todo!() }",
        "use serde_json::Error as Failure; fn run() -> Result<(), Failure> { todo!() }",
        "fn run<E>() -> Result<(), E> { todo!() }",
        "fn run() -> Result<(), MissingError> { todo!() }",
        "type Outcome<T> = Result<T, String>; fn run() -> Outcome<()> { todo!() }",
        "use std::result::Result as Outcome; type Failure = String; fn run() -> Outcome<(), \
         Failure> { todo!() }",
        "fn run() { use anyhow::Result as Outcome; fn local() -> Outcome<()> { todo!() } }",
    ] {
        assert!(
            reasons(source)
                .iter()
                .any(|reason| reason.contains("Result errors must use shared::Error")),
            "{source}: {:?}",
            reasons(source)
        );
    }
}

#[test]
fn rejects_layer_errors_derived_errors_and_custom_error_implementations() {
    for source in [
        "pub enum DomainError { Invalid }",
        "pub struct ValidationError;",
        "pub type PrivateError = String;",
        "#[derive(thiserror::Error)] enum Failure { Invalid }",
        "#[derive(Error)] struct Failure;",
        "impl std::error::Error for Failure {}",
        "mod nested { enum AnotherError { Broken } }",
        "#[cfg(test)] mod tests { enum TestError { Broken } }",
    ] {
        assert!(!reasons(source).is_empty(), "{source}");
    }
}

#[test]
fn accepts_only_the_transparent_api_response_wrapper() {
    let path = "src/api/error.rs";
    assert!(inspect(&[]).is_empty());
    for source in [
        "pub enum ApiError { Known(application::Error) }",
        "pub struct ApiError(pub String);",
        "#[derive(thiserror::Error)] pub struct ApiError(pub application::Error);",
        "pub struct ApiError(pub application::Error, pub String);",
    ] {
        let action = HTTP_ACTION.replace("pub struct ApiError(pub application::Error);", source);
        assert!(!inspect(&[(path, &action)]).is_empty(), "{source}");
    }
    assert!(
        !inspect(&[(
            "apps/domain/src/error.rs",
            "pub struct ApiError(pub application::Error);"
        )])
        .is_empty()
    );
}

#[test]
fn allows_api_wrapper_results_only_in_http_presentation() {
    let definition = (HTTP_ADAPTER, HTTP_ACTION);
    for source in [
        "use crate::api::error::ApiError; fn run() -> Result<(), ApiError> { todo!() }",
        "use super::super::error::ApiError; fn run() -> Result<(), ApiError> { todo!() }",
    ] {
        assert!(
            inspect(&[definition, ("src/api/handlers/example.rs", source)]).is_empty(),
            "{source}"
        );
    }
    let source = "use app::api::error::ApiError; fn run() -> Result<(), ApiError> { todo!() }";
    for path in
        ["src/middlewares/auth.rs", "src/internal/mappers/example.rs", "apps/domain/src/example.rs"]
    {
        assert!(!inspect(&[definition, (path, source)]).is_empty(), "{path}");
    }
}

#[test]
fn allows_tonic_only_at_rpc_response_boundaries() {
    let source =
        "use tonic::{Response, Status}; fn call() -> Result<Response<Value>, Status> { todo!() }";
    assert!(inspect(&[("src/internal/handlers/crf/example.rs", source)]).is_empty());
    for path in ["src/internal/mappers/example.rs", "apps/domain/src/example.rs"] {
        assert!(!inspect(&[(path, source)]).is_empty(), "{path}");
    }
    assert!(
        !inspect(&[(
            "src/internal/handlers/example.rs",
            "fn helper() -> Result<(), tonic::Status> { todo!() }"
        )])
        .is_empty()
    );
    assert!(
        !inspect(&[(
            "src/internal/mappers/example.rs",
            "impl TryFrom<()> for Value { type Error = tonic::Status; fn try_from(_: ()) -> \
             Result<Self, Self::Error> { todo!() } }"
        )])
        .is_empty()
    );
}

#[test]
fn adding_a_shared_variant_requires_actions_in_every_existing_transport_mapper() {
    let errors = inspect(&[
        (
            SHARED_FILE,
            "pub enum Error { StudyDoesNotExist, Unexpected(String), NewFailure }",
        ),
        (GRPC_MAPPER, GRPC_ACTION),
    ]);
    for path in [HTTP_ADAPTER, GRPC_MAPPER] {
        assert!(
            errors.iter().any(|error| error.path == path && error.reason.contains("NewFailure")),
            "{errors:?}"
        );
    }
}

#[test]
fn rest_only_service_does_not_require_an_internal_grpc_mapper() {
    let sources = sources(&[]);
    assert!(sources.iter().all(|source| source.path != GRPC_MAPPER));
    assert!(violations(&sources).is_empty());
    let errors = inspect(&[(
        SHARED_FILE,
        "pub enum Error { StudyDoesNotExist, Unexpected(String), NewFailure }",
    )]);
    assert!(
        errors
            .iter()
            .any(|error| error.path == HTTP_ADAPTER && error.reason.contains("NewFailure"))
    );
    assert!(errors.iter().all(|error| error.path != GRPC_MAPPER));
}

#[test]
fn present_grpc_mappers_require_explicit_defined_error_actions() {
    assert!(inspect(&[(GRPC_MAPPER, GRPC_ACTION)]).is_empty());
    let missing = "fn map_error(error: application::Error) -> tonic::Status { match error { \
                   application::Error::StudyDoesNotExist => todo!() } }";
    assert!(inspect(&[(GRPC_MAPPER, missing)]).iter().any(|error| {
        error.path == GRPC_MAPPER
            && error.reason.contains("missing explicit unguarded variants: Unexpected")
    }));
    let undefined = "fn map_error(error: application::Error) -> tonic::Status { match error { \
                     application::Error::StudyDoesNotExist => todo!(), \
                     application::Error::Unexpected(_) => todo!(), application::Error::NotDefined \
                     => todo!() } }";
    assert!(inspect(&[(GRPC_MAPPER, undefined)]).iter().any(|error| {
        error.path == GRPC_MAPPER
            && error.reason.contains("undefined shared error variant NotDefined")
    }));
}

#[test]
fn rejects_wildcard_and_binding_arms_even_when_other_variants_are_covered() {
    for pattern in ["_", "error", "error @ _"] {
        let source = format!(
            "use shared::Error; fn action(error: Error) {{ match error {{ \
             Error::StudyDoesNotExist => (), {pattern} => () }} }}"
        );
        let errors = reasons(&source);
        assert!(
            errors.iter().any(|reason| reason.contains("catch-all")),
            "{source}: {errors:?}"
        );
        assert!(
            errors.iter().any(|reason| reason.contains("Unexpected")),
            "{source}: {errors:?}"
        );
    }
    assert!(
        reasons(
            "use shared::Error; fn action(error: Error) { match error { Error::StudyDoesNotExist \
             => (), item @ Error::Unexpected(_) => () } }"
        )
        .is_empty()
    );
}

#[test]
fn detects_error_matches_through_typed_aliases_locals_and_function_returns() {
    for source in [
        "use shared::Error as Failure; fn action(error: Failure) { match &error { _ => () } }",
        "fn action(error: shared::Error) { let another = error; match another { _ => () } }",
        "fn create() -> shared::Error { todo!() } fn action() { match create() { _ => () } }",
        "fn action() { let error = shared::Error::Unexpected(String::new()); match error { _ => \
         () } }",
        "fn action() { let error: shared::Error = todo!(); match error { _ => () } }",
        "fn action() { let callback = |error: shared::Error| match error { _ => () }; }",
        "use shared::Error; impl Error { fn action(&self) { match self { _ => () } } }",
    ] {
        assert!(
            reasons(source).iter().any(|reason| reason.contains("catch-all")),
            "{source}: {:?}",
            reasons(source)
        );
    }
}

#[test]
fn accepts_glob_variants_renamed_variants_references_and_or_patterns() {
    for source in [
        "use shared::Error::*; fn action(error: shared::Error) { match error { StudyDoesNotExist \
         => (), Unexpected(_) => () } }",
        "use shared::Error::{StudyDoesNotExist as Missing, Unexpected as Busy}; fn action(error: \
         shared::Error) { match error { Missing => (), Busy(_) => () } }",
        "use shared::Error; fn action(error: Error) { match error { Error::StudyDoesNotExist | \
         Error::Unexpected(_) => () } }",
        "use shared::Error; fn action(error: &Error) { match error { &Error::StudyDoesNotExist => \
         (), &Error::Unexpected(_) => () } }",
    ] {
        assert!(
            reasons(source).is_empty(),
            "{source}: {:?}",
            reasons(source)
        );
    }
}

#[test]
fn guarded_variants_do_not_count_as_complete_handling() {
    let errors = reasons(
        "use shared::Error; fn action(error: Error) { match error { Error::StudyDoesNotExist if \
         true => (), Error::Unexpected(_) => () } }",
    );
    assert!(
        errors.iter().any(|reason| {
            reason.contains("missing explicit unguarded variants: StudyDoesNotExist")
        }),
        "{errors:?}"
    );
}

#[test]
fn requires_a_real_production_canonical_enum_and_transport_mappers() {
    for source in ["", "pub struct Error;", "#[cfg(test)] pub enum Error { Unexpected }"] {
        assert!(!inspect(&[(SHARED_FILE, source)]).is_empty(), "{source}");
    }
    assert!(
        !inspect(&[(
            SHARED_FILE,
            "#[non_exhaustive] pub enum Error { StudyDoesNotExist, Unexpected(String) }"
        )])
        .is_empty()
    );
    assert!(
        !inspect(&[(
            SHARED_FILE,
            "enum Error { StudyDoesNotExist, Unexpected(String) }"
        )])
        .is_empty()
    );
    let sources = sources(&[]);
    let without_http: Vec<_> =
        sources.into_iter().filter(|source| source.path != HTTP_ADAPTER).collect();
    assert!(violations(&without_http).iter().any(|violation| violation.path == HTTP_ADAPTER));
    for path in [HTTP_ADAPTER, GRPC_MAPPER] {
        assert!(!inspect(&[(path, "fn map(error: application::Error) { todo!() }")]).is_empty());
    }
}

#[test]
fn tests_can_use_provider_setup_results_but_cannot_define_new_errors() {
    assert!(
        reasons("#[cfg(test)] mod tests { fn setup() -> Result<(), String> { todo!() } }")
            .is_empty()
    );
    assert!(reasons("#[test] fn setup() -> anyhow::Result<()> { todo!() }").is_empty());
    assert!(reasons("#[cfg(test)] type Setup = anyhow::Result<()>;").is_empty());
    assert!(
        inspect(&[
            (
                "apps/domain/src/lib.rs",
                "pub use shared::Error; #[cfg(test)] #[path = \"tests/unit.rs\"] mod unit;"
            ),
            (
                "apps/domain/src/tests/unit.rs",
                "fn setup() -> anyhow::Result<()> { todo!() }"
            )
        ])
        .is_empty()
    );
    assert!(
        !reasons(
            "#[cfg(any(test, feature = \"runtime\"))] fn setup() -> Result<(), String> { todo!() }"
        )
        .is_empty()
    );
    assert!(!reasons("#[cfg(test)] struct SetupError;").is_empty());
}

#[test]
fn undefined_shared_variant_references_require_definition_and_actions() {
    for source in [
        "fn run() -> shared::Error { shared::Error::NotDefined }",
        "use shared::Error as Failure; fn run() -> Failure { Failure::NotDefined }",
        "use shared::Error::NotDefined as Missing; fn run() -> shared::Error { Missing }",
    ] {
        assert!(
            reasons(source)
                .iter()
                .any(|reason| reason.contains("undefined shared error variant NotDefined")),
            "{source}: {:?}",
            reasons(source)
        );
    }
    for source in [
        "fn run(error: shared::Error) -> shared::Error { shared::Error::from(error) }",
        "fn run(error: shared::Error) { let error = shared::Error::try_from(error); }",
    ] {
        assert!(
            reasons(source).is_empty(),
            "{source}: {:?}",
            reasons(source)
        );
    }
}

#[test]
fn standard_from_str_err_associated_type_uses_shared_error() {
    assert!(
        reasons(
            "use shared::Error; impl std::str::FromStr for Value { type Err = Error; fn \
             from_str(_: &str) -> Result<Self, Self::Err> { todo!() } }"
        )
        .is_empty()
    );
    assert!(
        !reasons(
            "impl std::str::FromStr for Value { type Err = String; fn from_str(_: &str) -> \
             Result<Self, Self::Err> { todo!() } }"
        )
        .is_empty()
    );
}

#[test]
fn clone_borrow_and_dereference_preserve_error_match_enforcement() {
    for expression in ["error.clone()", "*error", "error.borrow()", "(error).clone()"] {
        let source =
            format!("fn handle(error: &shared::Error) {{ match {expression} {{ _ => () }} }}");
        assert!(
            reasons(&source).iter().any(|reason| reason.contains("catch-all")),
            "{source}: {:?}",
            reasons(&source)
        );
    }
}

#[test]
fn test_exemptions_follow_module_reachability_instead_of_directory_names() {
    let runtime = (
        "apps/domain/src/tests/runtime.rs",
        "pub fn runtime() -> Result<(), String> { todo!() }",
    );
    let production = (
        "apps/domain/src/lib.rs",
        "pub use shared::Error; #[path = \"tests/runtime.rs\"] pub mod runtime;",
    );
    assert!(inspect(&[production, runtime]).iter().any(|violation| {
        violation.path == runtime.0
            && violation.reason.contains("Result errors must use shared::Error")
    }));
    let testing = (
        "apps/domain/src/lib.rs",
        "pub use shared::Error; #[cfg(test)] #[path = \"tests/runtime.rs\"] mod runtime;",
    );
    assert!(inspect(&[testing, runtime]).is_empty());
    let mixed = (
        "apps/domain/src/lib.rs",
        "pub use shared::Error; #[cfg(test)] #[path = \"tests/runtime.rs\"] mod fixture; #[path = \
         \"tests/runtime.rs\"] mod runtime;",
    );
    assert!(!inspect(&[mixed, runtime]).is_empty());
}

#[test]
fn unrelated_exhaustive_matches_cannot_replace_real_transport_actions() {
    for (path, action) in [
        (
            HTTP_ADAPTER,
            "pub struct ApiError(pub application::Error); impl axum::response::IntoResponse for \
             ApiError { fn into_response(self) -> axum::response::Response { match Some(self.0) { \
             _ => todo!() } } }",
        ),
        (
            GRPC_MAPPER,
            "fn map_error(error: application::Error) -> tonic::Status { match Some(error) { _ => \
             todo!() } }",
        ),
    ] {
        let source = format!(
            "{action} fn unrelated(error: application::Error) {{ match error {{ \
             application::Error::StudyDoesNotExist => (), application::Error::Unexpected(_) => () \
             }} }}"
        );
        assert!(
            inspect(&[(path, &source)]).iter().any(|violation| {
                violation.path == path
                    && violation.reason.contains("central transport error adapter")
            }),
            "{path}: {:?}",
            inspect(&[(path, &source)])
        );
    }
    let same_function = "fn map_error(error: application::Error) -> tonic::Status { let decoy = \
                         application::Error::StudyDoesNotExist; match decoy { \
                         application::Error::StudyDoesNotExist => (), \
                         application::Error::Unexpected(_) => () }; todo!() }";
    assert!(
        inspect(&[(GRPC_MAPPER, same_function)]).iter().any(|violation| {
            violation.path == GRPC_MAPPER
                && violation.reason.contains("central transport error adapter")
        })
    );
}

#[test]
fn both_transport_mappers_reject_wildcard_and_binding_error_actions() {
    for pattern in ["_", "error"] {
        let http = format!(
            "pub struct ApiError(pub application::Error); impl axum::response::IntoResponse for \
             ApiError {{ fn into_response(self) -> axum::response::Response {{ match self.0 {{ \
             {pattern} => todo!() }} }} }}"
        );
        let grpc = format!(
            "fn map_error(error: application::Error) -> tonic::Status {{ match error {{ {pattern} \
             => todo!() }} }}"
        );
        for (path, source) in [(HTTP_ADAPTER, http), (GRPC_MAPPER, grpc)] {
            assert!(inspect(&[(path, &source)]).iter().any(|violation| {
                violation.path == path && violation.reason.contains("catch-all")
            }));
        }
    }
}

#[test]
fn inspects_macro_templates_and_function_item_macro_arguments() {
    for source in [
        "macro_rules! helper { () => { fn run() -> Result<(), String> { todo!() } }; }",
        "macro_rules! helper { ($name:ident) => { fn $name() -> Result<(), String> { todo!() } }; \
         }",
        "macro_rules! helper { ($($name:ident),*) => { $(fn $name() -> Result<(), String> { \
         todo!() })* }; }",
        "macro_rules! helper { () => { pub enum NewError { Broken } }; }",
        "wrapper! { fn run() -> Result<(), String> { todo!() } }",
    ] {
        assert!(!reasons(source).is_empty(), "{source}");
    }
    assert!(
        reasons(
            "macro_rules! helper { ($name:ident) => { fn $name() -> Result<(), $crate::Error> { \
             todo!() } }; }"
        )
        .is_empty()
    );
}

#[test]
fn inferred_collect_results_do_not_introduce_another_error_contract() {
    assert!(
        reasons(
            "fn action() -> Result<(), shared::Error> { let result = values.collect::<Result<_, \
             _>>()?; Ok(()) }"
        )
        .is_empty()
    );
    assert!(
        !reasons(
            "fn action() -> Result<(), shared::Error> { let result = values.collect::<Result<_, \
             String>>()?; Ok(()) }"
        )
        .is_empty()
    );
}

#[test]
fn scans_service_sources_only_and_reports_positions_without_writing() {
    assert!(!service_source("tools/rslint/src/main.rs"));
    assert!(!service_source("build.rs"));
    assert!(service_source("src/main.rs"));
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(Command::new("git").args(["init", "--quiet"]).arg(root).status().unwrap().success());
    let files = [
        (SHARED_FILE, ENUM),
        (
            "apps/shared/src/lib.rs",
            "pub mod errors; pub use errors::Error;",
        ),
        ("apps/application/src/lib.rs", "pub use shared::Error;"),
        (HTTP_ADAPTER, HTTP_ACTION),
        (GRPC_MAPPER, GRPC_ACTION),
        (
            "src/main.rs",
            "\nfn failure() -> Result<(), String> { todo!() }",
        ),
        (
            "tools/helper/src/main.rs",
            "fn failure() -> Result<(), String> { todo!() }",
        ),
    ];
    for (path, source) in files {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), source).unwrap();
    }
    assert_eq!(check_errors(root).unwrap(), 1);
    for (path, source) in files {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), source);
    }
    let result = inspect(&[(
        "src/main.rs",
        "\nfn failure() -> Result<(), String> { todo!() }",
    )]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].line, 2);
    assert!(result[0].column > 1);
}
