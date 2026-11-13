mod create_item;
mod delete_item;
mod delete_items;
mod get_item;
mod list_items;
mod update_item;

use tonic::{
    Request,
    Response,
    Status,
};

use crate::internal::{
    pb::item::v1::{
        CreateItemRequest,
        DeleteItemReply,
        DeleteItemRequest,
        DeleteItemsReply,
        DeleteItemsRequest,
        GetItemRequest,
        ItemReply,
        ListItemsReply,
        ListItemsRequest,
        UpdateItemRequest,
        item_internal_server::ItemInternal,
    },
    service::ItemGrpcService,
};

#[tonic::async_trait]
impl ItemInternal for ItemGrpcService {
    async fn create_item(
        &self,
        request: Request<CreateItemRequest>,
    ) -> Result<Response<ItemReply>, Status> {
        create_item::handle(self, request).await
    }

    async fn get_item(
        &self,
        request: Request<GetItemRequest>,
    ) -> Result<Response<ItemReply>, Status> {
        get_item::handle(self, request).await
    }

    async fn update_item(
        &self,
        request: Request<UpdateItemRequest>,
    ) -> Result<Response<ItemReply>, Status> {
        update_item::handle(self, request).await
    }

    async fn delete_item(
        &self,
        request: Request<DeleteItemRequest>,
    ) -> Result<Response<DeleteItemReply>, Status> {
        delete_item::handle(self, request).await
    }

    async fn delete_items(
        &self,
        request: Request<DeleteItemsRequest>,
    ) -> Result<Response<DeleteItemsReply>, Status> {
        delete_items::handle(self, request).await
    }

    async fn list_items(
        &self,
        request: Request<ListItemsRequest>,
    ) -> Result<Response<ListItemsReply>, Status> {
        list_items::handle(self, request).await
    }
}
