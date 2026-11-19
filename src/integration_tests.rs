#[path = "../tests/host/fixtures.rs"]
mod fixtures;
#[path = "../tests/host/grpc_tests.rs"]
mod grpc_tests;
#[path = "../tests/host/repository.rs"]
mod repository;
#[path = "../tests/host/router_tests.rs"]
mod router_tests;

use fixtures::{
    app_with_repository,
    test_app,
};
