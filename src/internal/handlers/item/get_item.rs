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
        GetItemRequest,
        ItemReply,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    request: Request<GetItemRequest>,
) -> Result<Response<ItemReply>, Status> {
    let input = mapper::to_get(request.into_inner()).map_err(map_error)?;
    let item = service.get_item().execute(input).await.map_err(map_error)?;
    Ok(Response::new(mapper::to_reply(&item)))
}
