use application::UpdateItemCommand;
use tonic::{
    Request,
    Response,
    Status,
};
use uuid::Uuid;

use crate::internal::{
    error::map_error,
    pb::item::v1::{
        ItemReply,
        UpdateItemRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<UpdateItemRequest>,
) -> Result<Response<ItemReply>, Status> {
    let request = request.into_inner();
    let id =
        Uuid::parse_str(&request.id).map_err(|_| Status::invalid_argument("invalid item id"))?;
    let command = UpdateItemCommand {
        id,
        title: request.title,
    };
    service
        .update_item()
        .execute(command)
        .await
        .map(ItemReply::from)
        .map(Response::new)
        .map_err(map_error)
}
