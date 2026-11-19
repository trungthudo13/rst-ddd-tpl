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
        ItemReply,
        UpdateItemRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<UpdateItemRequest>,
) -> Result<Response<ItemReply>, Status> {
    let input = mapper::to_update(request.into_inner()).map_err(map_error)?;
    let item = service.update_item().execute(input).await.map_err(map_error)?;
    Ok(Response::new(mapper::to_reply(&item)))
}
