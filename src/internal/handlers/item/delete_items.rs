use tonic::{
    Request,
    Response,
    Status,
};

use crate::internal::{
    mappers::{
        errors::map_error,
        item as mapper,
    },
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
    let command = mapper::to_delete_many(request.into_inner()).map_err(map_error)?;
    service.delete_items().execute(command).await.map_err(map_error)?;
    Ok(Response::new(mapper::deleted_many()))
}
