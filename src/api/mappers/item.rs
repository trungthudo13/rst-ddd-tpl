use application::{
    CreateItemCommand,
    DeleteItemCommand,
    DeleteItemsCommand,
    Error,
    GetItemQuery,
    UpdateItemCommand,
};
use domain::{
    Item,
    ItemSnapshot,
    mappers::item as domain_item,
};
use uuid::Uuid;

use crate::api::schemas::item::{
    CreateItemRequest,
    DeleteItemsRequest,
    ItemView,
    UpdateItemRequest,
};

pub fn to_create(request: CreateItemRequest) -> CreateItemCommand {
    CreateItemCommand {
        title: request.title,
    }
}

pub fn from_create(command: CreateItemCommand) -> CreateItemRequest {
    CreateItemRequest {
        title: command.title,
    }
}

pub fn to_update(
    id: &str,
    request: UpdateItemRequest,
) -> Result<UpdateItemCommand, Error> {
    let input = UpdateItemCommand {
        id: parse_id(id)?,
        title: request.title,
    };
    Ok(input)
}

pub fn from_update(command: UpdateItemCommand) -> (String, UpdateItemRequest) {
    let request = UpdateItemRequest {
        title: command.title,
    };
    (command.id.to_string(), request)
}

pub fn to_delete(id: &str) -> Result<DeleteItemCommand, Error> {
    let input = DeleteItemCommand {
        id: parse_id(id)?,
    };
    Ok(input)
}

pub fn from_delete(command: DeleteItemCommand) -> String {
    command.id.to_string()
}

pub fn to_delete_many(request: DeleteItemsRequest) -> Result<DeleteItemsCommand, Error> {
    let ids = request.ids.iter().map(|id| parse_id(id)).collect::<Result<Vec<_>, _>>()?;
    let input = DeleteItemsCommand {
        ids,
    };
    Ok(input)
}

pub fn from_delete_many(command: DeleteItemsCommand) -> DeleteItemsRequest {
    DeleteItemsRequest {
        ids: command.ids.into_iter().map(|id| id.to_string()).collect(),
    }
}

pub fn to_get(id: &str) -> Result<GetItemQuery, Error> {
    let input = GetItemQuery {
        id: parse_id(id)?,
    };
    Ok(input)
}

pub fn from_get(query: GetItemQuery) -> String {
    query.id.to_string()
}

pub fn to_view(item: &Item) -> ItemView {
    let snapshot = domain_item::to_snapshot(item);
    ItemView {
        id: snapshot.id.to_string(),
        title: snapshot.title,
        created_at: snapshot.created_at,
    }
}

pub fn from_view(view: ItemView) -> Result<Item, Error> {
    let snapshot = ItemSnapshot {
        id: parse_id(&view.id)?,
        title: view.title,
        created_at: view.created_at,
    };
    domain_item::from_snapshot(snapshot)
}

fn parse_id(id: &str) -> Result<Uuid, Error> {
    Uuid::parse_str(id).map_err(|_| Error::InvalidId)
}
