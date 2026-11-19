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
        DeleteItemReply,
        DeleteItemRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<DeleteItemRequest>,
) -> Result<Response<DeleteItemReply>, Status> {
    let command = mapper::to_delete(request.into_inner()).map_err(map_error)?;
    service.delete_item().execute(command).await.map_err(map_error)?;
    Ok(Response::new(mapper::deleted()))
}
