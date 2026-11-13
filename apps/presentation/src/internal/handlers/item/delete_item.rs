use application::DeleteItemCommand;
use tonic::{
    Request,
    Response,
    Status,
};
use uuid::Uuid;

use crate::internal::{
    error::map_error,
    pb::item::v1::{
        DeleteItemReply,
        DeleteItemRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<DeleteItemRequest>,
) -> Result<Response<DeleteItemReply>, Status> {
    let id = Uuid::parse_str(&request.into_inner().id)
        .map_err(|_| Status::invalid_argument("invalid item id"))?;
    service
        .delete_item()
        .execute(DeleteItemCommand {
            id,
        })
        .await
        .map_err(map_error)?;
    Ok(Response::new(DeleteItemReply {}))
}
