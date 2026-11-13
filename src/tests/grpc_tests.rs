use std::{
    sync::Arc,
    time::Duration,
};

use presentation::internal::pb::item::v1::{
    CreateItemRequest,
    DeleteItemRequest,
    DeleteItemsRequest,
    GetItemRequest,
    ListItemsRequest,
    UpdateItemRequest,
    item_internal_client::ItemInternalClient,
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
use crate::server::serve_with_shutdown;

async fn start(
    app: axum::Router
) -> (
    ItemInternalClient<Channel>,
    oneshot::Sender<()>,
    JoinHandle<anyhow::Result<()>>,
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
    (ItemInternalClient::new(channel), shutdown, server)
}

async fn stop(
    shutdown: oneshot::Sender<()>,
    server: JoinHandle<anyhow::Result<()>>,
) {
    shutdown.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap().unwrap();
}

#[tokio::test]
async fn grpc_crud_uses_generated_client_over_combined_listener() {
    let (mut client, shutdown, server) = start(test_app(false)).await;
    let item = client
        .create_item(CreateItemRequest {
            title: " first ".into(),
        })
        .await
        .unwrap()
        .into_inner();
    Uuid::parse_str(&item.id).unwrap();
    assert_eq!(item.title, "first");
    assert!(item.created_at > 0);
    assert_eq!(
        client
            .get_item(GetItemRequest {
                id: item.id.clone()
            })
            .await
            .unwrap()
            .into_inner(),
        item
    );
    assert_eq!(
        client.list_items(ListItemsRequest {}).await.unwrap().into_inner().items,
        vec![item.clone()]
    );

    let updated = client
        .update_item(UpdateItemRequest {
            id: item.id.clone(),
            title: " updated ".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(updated.id, item.id);
    assert_eq!(updated.created_at, item.created_at);
    assert_eq!(updated.title, "updated");
    let invalid = client
        .update_item(UpdateItemRequest {
            id: item.id.clone(),
            title: "  ".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(invalid.code(), Code::InvalidArgument);
    assert_eq!(
        client
            .get_item(GetItemRequest {
                id: item.id.clone()
            })
            .await
            .unwrap()
            .into_inner(),
        updated
    );

    client
        .delete_item(DeleteItemRequest {
            id: item.id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .get_item(GetItemRequest {
                id: item.id.clone()
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .update_item(UpdateItemRequest {
                id: item.id.clone(),
                title: "missing".into()
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .delete_item(DeleteItemRequest {
                id: item.id.clone()
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert!(client.list_items(ListItemsRequest {}).await.unwrap().into_inner().items.is_empty());

    let other = client
        .create_item(CreateItemRequest {
            title: "other".into(),
        })
        .await
        .unwrap()
        .into_inner();
    let invalid = client
        .delete_items(DeleteItemsRequest {
            ids: vec![other.id.clone(), "invalid".into()],
        })
        .await
        .unwrap_err();
    assert_eq!(invalid.code(), Code::InvalidArgument);
    client
        .delete_items(DeleteItemsRequest {
            ids: vec![],
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .get_item(GetItemRequest {
                id: other.id.clone()
            })
            .await
            .unwrap()
            .into_inner(),
        other
    );
    client
        .delete_items(DeleteItemsRequest {
            ids: vec![other.id.clone(), other.id, Uuid::new_v4().to_string()],
        })
        .await
        .unwrap();
    assert!(client.list_items(ListItemsRequest {}).await.unwrap().into_inner().items.is_empty());
    drop(client);
    stop(shutdown, server).await;
}

#[tokio::test]
async fn grpc_maps_validation_and_hides_provider_details() {
    let (mut client, shutdown, server) = start(test_app(false)).await;
    assert_eq!(
        client
            .create_item(CreateItemRequest {
                title: " ".into()
            })
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_eq!(
        client
            .get_item(GetItemRequest {
                id: "invalid".into()
            })
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_eq!(
        client
            .update_item(UpdateItemRequest {
                id: "invalid".into(),
                title: "valid".into()
            })
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_eq!(
        client
            .delete_item(DeleteItemRequest {
                id: "invalid".into()
            })
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    drop(client);
    stop(shutdown, server).await;

    let app = app_with_repository(Arc::new(TestRepository::unavailable()), false);
    let (mut client, shutdown, server) = start(app).await;
    let id = Uuid::new_v4().to_string();
    let errors = [
        client
            .create_item(CreateItemRequest {
                title: "valid".into(),
            })
            .await
            .unwrap_err(),
        client
            .get_item(GetItemRequest {
                id: id.clone(),
            })
            .await
            .unwrap_err(),
        client.list_items(ListItemsRequest {}).await.unwrap_err(),
        client
            .update_item(UpdateItemRequest {
                id: id.clone(),
                title: "valid".into(),
            })
            .await
            .unwrap_err(),
        client
            .delete_item(DeleteItemRequest {
                id: id.clone(),
            })
            .await
            .unwrap_err(),
        client
            .delete_items(DeleteItemsRequest {
                ids: vec![id],
            })
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code(), Code::Internal);
        assert_eq!(error.message(), "internal server error");
    }
    drop(client);
    stop(shutdown, server).await;
}
