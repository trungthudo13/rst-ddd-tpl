use application::DeleteItemsCommand;
use tonic::{
    Request,
    Response,
    Status,
};
use uuid::Uuid;

use crate::internal::{
    error::map_error,
    pb::item::v1::{
        DeleteItemsReply,
        DeleteItemsRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<DeleteItemsRequest>,
) -> Result<Response<DeleteItemsReply>, Status> {
    let ids = request
        .into_inner()
        .ids
        .iter()
        .map(|id| Uuid::parse_str(id).map_err(|_| Status::invalid_argument("invalid item id")))
        .collect::<Result<Vec<_>, _>>()?;
    let command = DeleteItemsCommand {
        ids,
    };
    service.delete_items().execute(command).await.map_err(map_error)?;
    Ok(Response::new(DeleteItemsReply {}))
}
