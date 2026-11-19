use application::{
    CreateItemCommand,
    DeleteItemCommand,
    DeleteItemsCommand,
    Error,
    GetItemQuery,
    UpdateItemCommand,
};
use domain::Item;
use uuid::Uuid;

use crate::{
    api::{
        mappers::item as rest,
        schemas::item::{
            DeleteItemsRequest,
            ItemView,
            UpdateItemRequest,
        },
    },
    internal::{
        mappers::item as grpc,
        pb::item::v1 as pb,
    },
};

#[test]
fn response_round_trips_preserve_identity_unicode_and_timestamps() {
    let item = Item::new(Uuid::nil(), "résumé 🦀".into(), i64::MAX).unwrap();
    let view = rest::to_view(&item);
    assert_eq!(rest::from_view(view).unwrap(), item);
    assert_eq!(grpc::from_reply(grpc::to_reply(&item)).unwrap(), item);
}

#[test]
fn reverse_responses_reject_malformed_identity_and_invalid_domain_values() {
    let invalid = ItemView {
        id: "invalid".into(),
        title: "valid".into(),
        created_at: 1,
    };
    assert_eq!(rest::from_view(invalid), Err(Error::InvalidId));
    let invalid = pb::ItemReply {
        id: Uuid::nil().to_string(),
        title: " \t".into(),
        created_at: 1,
    };
    assert_eq!(grpc::from_reply(invalid), Err(Error::EmptyTitle));
}

#[test]
fn request_round_trips_keep_explicit_path_context_and_bulk_order() {
    let id = Uuid::nil();
    let create = CreateItemCommand {
        title: "  unchanged payload  ".into(),
    };
    assert_eq!(rest::to_create(rest::from_create(create.clone())), create);
    assert_eq!(grpc::to_create(grpc::from_create(create.clone())), create);
    let update = UpdateItemCommand {
        id,
        title: "updated".into(),
    };
    let (path, request) = rest::from_update(update.clone());
    assert_eq!(rest::to_update(&path, request).unwrap(), update);
    assert_eq!(
        grpc::to_update(grpc::from_update(update.clone())).unwrap(),
        update
    );
    let delete = DeleteItemCommand {
        id,
    };
    assert_eq!(rest::to_delete(&rest::from_delete(delete)).unwrap(), delete);
    assert_eq!(grpc::to_delete(grpc::from_delete(delete)).unwrap(), delete);
    let get = GetItemQuery {
        id,
    };
    assert_eq!(rest::to_get(&rest::from_get(get)).unwrap(), get);
    assert_eq!(grpc::to_get(grpc::from_get(get)).unwrap(), get);
    let bulk = DeleteItemsCommand {
        ids: vec![id, Uuid::from_u128(1), id],
    };
    assert_eq!(
        rest::to_delete_many(rest::from_delete_many(bulk.clone())).unwrap(),
        bulk
    );
    assert_eq!(
        grpc::to_delete_many(grpc::from_delete_many(bulk.clone())).unwrap(),
        bulk
    );
}

#[test]
fn invalid_identifiers_fail_before_a_command_can_be_executed() {
    assert_eq!(rest::to_get("invalid"), Err(Error::InvalidId));
    assert_eq!(rest::to_delete("invalid"), Err(Error::InvalidId));
    let update = UpdateItemRequest {
        title: "valid".into(),
    };
    assert_eq!(rest::to_update("invalid", update), Err(Error::InvalidId));
    let bulk = DeleteItemsRequest {
        ids: vec![Uuid::nil().to_string(), "invalid".into()],
    };
    assert_eq!(rest::to_delete_many(bulk), Err(Error::InvalidId));
    let get = pb::GetItemRequest {
        id: "invalid".into(),
    };
    assert_eq!(grpc::to_get(get), Err(Error::InvalidId));
    let delete = pb::DeleteItemRequest {
        id: "invalid".into(),
    };
    assert_eq!(grpc::to_delete(delete), Err(Error::InvalidId));
    let update = pb::UpdateItemRequest {
        id: "invalid".into(),
        title: "valid".into(),
    };
    assert_eq!(grpc::to_update(update), Err(Error::InvalidId));
    let bulk = pb::DeleteItemsRequest {
        ids: vec![Uuid::nil().to_string(), "invalid".into()],
    };
    assert_eq!(grpc::to_delete_many(bulk), Err(Error::InvalidId));
}
