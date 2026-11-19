use std::{
    sync::Arc,
    time::Duration,
};

use tokio::{
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};
use tonic::{
    Code,
    transport::{
        Channel,
        Endpoint,
    },
};
use uuid::Uuid;

use super::{
    app_with_repository,
    repository::TestRepository,
    test_app,
};
use crate::internal::pb::item::v1::{
    CreateItemRequest,
    DeleteItemRequest,
    DeleteItemsRequest,
    GetItemRequest,
    ListItemsRequest,
    UpdateItemRequest,
    item_internal_client::ItemInternalClient,
};
use crate::server::serve_with_shutdown;

async fn start(
    app: axum::Router,
    prefix: &str,
) -> (
    ItemInternalClient<Channel>,
    oneshot::Sender<()>,
    JoinHandle<Result<(), shared::Error>>,
) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown, receiver) = oneshot::channel();
    let server = tokio::spawn(serve_with_shutdown(listener, app, async {
        let _ = receiver.await;
    }));
    let channel = Endpoint::from_shared(format!("http://{address}"))
        .unwrap()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .connect()
        .await
        .unwrap();
    let origin = format!("http://{address}{prefix}").parse().unwrap();
    (
        ItemInternalClient::with_origin(channel, origin),
        shutdown,
        server,
    )
}

async fn stop(
    shutdown: oneshot::Sender<()>,
    server: JoinHandle<Result<(), shared::Error>>,
) {
    shutdown.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap().unwrap();
}

#[tokio::test]
async fn grpc_crud_uses_generated_client_over_combined_listener() {
    let (mut client, shutdown, server) = start(test_app(false), "/internal").await;
    let create_item_request = CreateItemRequest {
        title: " first ".into(),
    };
    let item = client.create_item(create_item_request).await.unwrap().into_inner();
    Uuid::parse_str(&item.id).unwrap();
    assert_eq!(item.title, "first");
    assert!(item.created_at > 0);
    let get_item_request = GetItemRequest {
        id: item.id.clone(),
    };
    assert_eq!(
        client.get_item(get_item_request).await.unwrap().into_inner(),
        item
    );
    let list_items_request = ListItemsRequest {};
    assert_eq!(
        client.list_items(list_items_request).await.unwrap().into_inner().items,
        vec![item.clone()]
    );

    let update_item_request = UpdateItemRequest {
        id: item.id.clone(),
        title: " updated ".into(),
    };
    let updated = client.update_item(update_item_request).await.unwrap().into_inner();
    assert_eq!(updated.id, item.id);
    assert_eq!(updated.created_at, item.created_at);
    assert_eq!(updated.title, "updated");
    let update_item_request = UpdateItemRequest {
        id: item.id.clone(),
        title: "  ".into(),
    };
    let invalid = client.update_item(update_item_request).await.unwrap_err();
    assert_eq!(invalid.code(), Code::InvalidArgument);
    let get_item_request = GetItemRequest {
        id: item.id.clone(),
    };
    assert_eq!(
        client.get_item(get_item_request).await.unwrap().into_inner(),
        updated
    );

    let delete_item_request = DeleteItemRequest {
        id: item.id.clone(),
    };
    client.delete_item(delete_item_request).await.unwrap();
    let get_item_request = GetItemRequest {
        id: item.id.clone(),
    };
    assert_eq!(
        client.get_item(get_item_request).await.unwrap_err().code(),
        Code::NotFound
    );
    let update_item_request = UpdateItemRequest {
        id: item.id.clone(),
        title: "missing".into(),
    };
    assert_eq!(
        client.update_item(update_item_request).await.unwrap_err().code(),
        Code::NotFound
    );
    let delete_item_request = DeleteItemRequest {
        id: item.id.clone(),
    };
    assert_eq!(
        client.delete_item(delete_item_request).await.unwrap_err().code(),
        Code::NotFound
    );
    let list_items_request = ListItemsRequest {};
    assert!(client.list_items(list_items_request).await.unwrap().into_inner().items.is_empty());

    let create_item_request = CreateItemRequest {
        title: "other".into(),
    };
    let other = client.create_item(create_item_request).await.unwrap().into_inner();
    let delete_items_request = DeleteItemsRequest {
        ids: vec![other.id.clone(), "invalid".into()],
    };
    let invalid = client.delete_items(delete_items_request).await.unwrap_err();
    assert_eq!(invalid.code(), Code::InvalidArgument);
    let delete_items_request = DeleteItemsRequest {
        ids: vec![],
    };
    client.delete_items(delete_items_request).await.unwrap();
    let get_item_request = GetItemRequest {
        id: other.id.clone(),
    };
    assert_eq!(
        client.get_item(get_item_request).await.unwrap().into_inner(),
        other
    );
    let delete_items_request = DeleteItemsRequest {
        ids: vec![other.id.clone(), other.id, Uuid::new_v4().to_string()],
    };
    client.delete_items(delete_items_request).await.unwrap();
    let list_items_request = ListItemsRequest {};
    assert!(client.list_items(list_items_request).await.unwrap().into_inner().items.is_empty());
    drop(client);
    stop(shutdown, server).await;
}

#[tokio::test]
async fn grpc_maps_validation_and_hides_provider_details() {
    let (mut client, shutdown, server) = start(test_app(false), "/internal").await;
    let create_item_request = CreateItemRequest {
        title: " ".into(),
    };
    assert_eq!(
        client.create_item(create_item_request).await.unwrap_err().code(),
        Code::InvalidArgument
    );
    let get_item_request = GetItemRequest {
        id: "invalid".into(),
    };
    assert_eq!(
        client.get_item(get_item_request).await.unwrap_err().code(),
        Code::InvalidArgument
    );
    let update_item_request = UpdateItemRequest {
        id: "invalid".into(),
        title: "valid".into(),
    };
    assert_eq!(
        client.update_item(update_item_request).await.unwrap_err().code(),
        Code::InvalidArgument
    );
    let delete_item_request = DeleteItemRequest {
        id: "invalid".into(),
    };
    assert_eq!(
        client.delete_item(delete_item_request).await.unwrap_err().code(),
        Code::InvalidArgument
    );
    drop(client);
    stop(shutdown, server).await;

    let app = app_with_repository(Arc::new(TestRepository::unavailable()), false);
    let (mut client, shutdown, server) = start(app, "/internal").await;
    let id = Uuid::new_v4().to_string();
    let create_item_request = CreateItemRequest {
        title: "valid".into(),
    };
    let get_item_request = GetItemRequest {
        id: id.clone(),
    };
    let list_items_request = ListItemsRequest {};
    let update_item_request = UpdateItemRequest {
        id: id.clone(),
        title: "valid".into(),
    };
    let delete_item_request = DeleteItemRequest {
        id: id.clone(),
    };
    let delete_items_request = DeleteItemsRequest {
        ids: vec![id],
    };
    let errors = [
        client.create_item(create_item_request).await.unwrap_err(),
        client.get_item(get_item_request).await.unwrap_err(),
        client.list_items(list_items_request).await.unwrap_err(),
        client.update_item(update_item_request).await.unwrap_err(),
        client.delete_item(delete_item_request).await.unwrap_err(),
        client.delete_items(delete_items_request).await.unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code(), Code::Internal);
        assert_eq!(error.message(), "internal server error");
    }
    drop(client);
    stop(shutdown, server).await;
}

#[tokio::test]
async fn grpc_routes_require_internal_prefix() {
    let (mut client, shutdown, server) = start(test_app(false), "").await;
    let list_items_request = ListItemsRequest {};
    let error = client.list_items(list_items_request).await.unwrap_err();
    assert_eq!(error.code(), Code::Unimplemented);
    drop(client);
    stop(shutdown, server).await;
}
