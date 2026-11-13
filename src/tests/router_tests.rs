use std::sync::Arc;

use axum::{
    body::{
        Body,
        to_bytes,
    },
    http::{
        Request,
        StatusCode,
    },
};
use serde_json::{
    Value,
    json,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::{
    app_with_repository,
    repository::TestRepository,
    test_app,
};

async fn request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    payload: Option<Value>,
) -> axum::response::Response {
    let builder = Request::builder().method(method).uri(uri);
    let request = match payload {
        | Some(payload) => {
            builder
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap()
        }
        | None => builder.body(Body::empty()).unwrap(),
    };
    app.clone().oneshot(request).await.unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_route_runs_through_combined_tower_stack() {
    let response = test_app(false)
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn openapi_and_swagger_ui_routes_are_available() {
    let app = test_app(true);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/docs/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("\"/items\""));
    assert!(body.contains("\"/items/bulk-delete\""));
    assert!(body.contains("\"/health\""));

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/docs/swagger/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("Swagger UI"));

    let response = app
        .oneshot(
            Request::builder().uri("/docs/swagger/swagger-ui.css").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn swagger_routes_are_unavailable_when_debug_is_disabled() {
    let app = test_app(false);

    for uri in ["/docs/openapi.json", "/docs/swagger/"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn item_update_delete_and_bulk_delete_routes_are_available() {
    let id = Uuid::new_v4();
    let app = test_app(false);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/items/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"updated"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/items/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/items/bulk-delete")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"ids":["a2eee96f-f875-4e4e-aeee-6718d610a796"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn rest_crud_preserves_identity_and_rejected_updates() {
    let app = test_app(false);
    let created = request(&app, "POST", "/items", Some(json!({"title": "  first  "}))).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let item = body_json(created).await;
    let id = item["id"].as_str().unwrap();
    Uuid::parse_str(id).unwrap();
    assert_eq!(item["title"], "first");
    assert!(item["created_at"].as_i64().unwrap() > 0);
    let uri = format!("/items/{id}");

    let found = request(&app, "GET", &uri, None).await;
    assert_eq!(found.status(), StatusCode::OK);
    assert_eq!(body_json(found).await, item);
    assert_eq!(
        body_json(request(&app, "GET", "/items", None).await).await,
        json!([item])
    );

    let updated = request(&app, "PATCH", &uri, Some(json!({"title": " updated "}))).await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = body_json(updated).await;
    assert_eq!(updated["id"], item["id"]);
    assert_eq!(updated["created_at"], item["created_at"]);
    assert_eq!(updated["title"], "updated");

    let invalid = request(&app, "PATCH", &uri, Some(json!({"title": "  "}))).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(invalid).await,
        json!({"error": "item title cannot be empty"})
    );
    assert_eq!(
        body_json(request(&app, "GET", &uri, None).await).await,
        updated
    );

    let deleted = request(&app, "DELETE", &uri, None).await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert!(to_bytes(deleted.into_body(), usize::MAX).await.unwrap().is_empty());
    assert_eq!(
        request(&app, "GET", &uri, None).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        body_json(request(&app, "GET", "/items", None).await).await,
        json!([])
    );
}

#[tokio::test]
async fn rest_bulk_delete_validates_all_ids_before_changing_state() {
    let app = test_app(false);
    let item =
        body_json(request(&app, "POST", "/items", Some(json!({"title": "first"}))).await).await;
    let invalid = request(
        &app,
        "POST",
        "/items/bulk-delete",
        Some(json!({
            "ids": [item["id"], "invalid"]
        })),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(request(&app, "GET", "/items", None).await).await,
        json!([item])
    );

    assert_eq!(
        request(&app, "POST", "/items/bulk-delete", Some(json!({"ids": []}))).await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        body_json(request(&app, "GET", "/items", None).await).await,
        json!([item])
    );
    let deleted = request(
        &app,
        "POST",
        "/items/bulk-delete",
        Some(json!({
            "ids": [item["id"], item["id"], Uuid::new_v4().to_string()]
        })),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        body_json(request(&app, "GET", "/items", None).await).await,
        json!([])
    );
}

#[tokio::test]
async fn rest_distinguishes_invalid_ids_missing_items_and_port_failures() {
    let app = test_app(false);
    for method in ["GET", "PATCH", "DELETE"] {
        let payload = (method == "PATCH").then(|| json!({"title": "updated"}));
        let invalid = request(&app, method, "/items/invalid", payload.clone()).await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_json(invalid).await,
            json!({"error": "invalid item id"})
        );
        let missing = request(&app, method, &format!("/items/{}", Uuid::new_v4()), payload).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(missing).await, json!({"error": "item not found"}));
    }
    let invalid = request(&app, "POST", "/items", Some(json!({"title": " "}))).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let app = app_with_repository(Arc::new(TestRepository::unavailable()), false);
    let uri = format!("/items/{}", Uuid::new_v4());
    for (method, path, payload) in [
        ("POST", "/items", Some(json!({"title": "first"}))),
        ("GET", "/items", None),
        ("GET", uri.as_str(), None),
        ("PATCH", uri.as_str(), Some(json!({"title": "updated"}))),
        ("DELETE", uri.as_str(), None),
        (
            "POST",
            "/items/bulk-delete",
            Some(json!({"ids": [Uuid::new_v4().to_string()]})),
        ),
    ] {
        let failed = request(&app, method, path, payload).await;
        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            body_json(failed).await,
            json!({"error": "internal server error"})
        );
    }
}
