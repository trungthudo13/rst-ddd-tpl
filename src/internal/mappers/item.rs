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

use crate::internal::pb::item::v1::{
    CreateItemRequest,
    DeleteItemReply,
    DeleteItemRequest,
    DeleteItemsReply,
    DeleteItemsRequest,
    GetItemRequest,
    ItemReply,
    ListItemsReply,
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

pub fn to_update(request: UpdateItemRequest) -> Result<UpdateItemCommand, Error> {
    let input = UpdateItemCommand {
        id: parse_id(&request.id)?,
        title: request.title,
    };
    Ok(input)
}

pub fn from_update(command: UpdateItemCommand) -> UpdateItemRequest {
    UpdateItemRequest {
        id: command.id.to_string(),
        title: command.title,
    }
}

pub fn to_delete(request: DeleteItemRequest) -> Result<DeleteItemCommand, Error> {
    let input = DeleteItemCommand {
        id: parse_id(&request.id)?,
    };
    Ok(input)
}

pub fn from_delete(command: DeleteItemCommand) -> DeleteItemRequest {
    DeleteItemRequest {
        id: command.id.to_string(),
    }
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

pub fn to_get(request: GetItemRequest) -> Result<GetItemQuery, Error> {
    let input = GetItemQuery {
        id: parse_id(&request.id)?,
    };
    Ok(input)
}

pub fn from_get(query: GetItemQuery) -> GetItemRequest {
    GetItemRequest {
        id: query.id.to_string(),
    }
}

pub fn to_reply(item: &Item) -> ItemReply {
    let snapshot = domain_item::to_snapshot(item);
    ItemReply {
        id: snapshot.id.to_string(),
        title: snapshot.title,
        created_at: snapshot.created_at,
    }
}

pub fn from_reply(reply: ItemReply) -> Result<Item, Error> {
    let snapshot = ItemSnapshot {
        id: parse_id(&reply.id)?,
        title: reply.title,
        created_at: reply.created_at,
    };
    domain_item::from_snapshot(snapshot)
}

pub fn list_reply(items: &[Item]) -> ListItemsReply {
    ListItemsReply {
        items: items.iter().map(to_reply).collect(),
    }
}

pub fn deleted() -> DeleteItemReply {
    DeleteItemReply {}
}

pub fn deleted_many() -> DeleteItemsReply {
    DeleteItemsReply {}
}

fn parse_id(id: &str) -> Result<Uuid, Error> {
    Uuid::parse_str(id).map_err(|_| Error::InvalidId)
}
