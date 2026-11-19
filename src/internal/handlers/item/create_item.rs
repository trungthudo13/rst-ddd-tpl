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
        CreateItemRequest,
        ItemReply,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<CreateItemRequest>,
) -> Result<Response<ItemReply>, Status> {
    let input = mapper::to_create(request.into_inner());
    let item = service.create_item().execute(input).await.map_err(map_error)?;
    Ok(Response::new(mapper::to_reply(&item)))
}
