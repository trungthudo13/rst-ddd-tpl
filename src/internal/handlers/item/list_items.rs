use application::GetItemsQuery;
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
        ListItemsReply,
        ListItemsRequest,
    },
    service::ItemGrpcService,
};

pub(super) async fn handle(
    service: &ItemGrpcService,
    _request: Request<ListItemsRequest>,
) -> Result<Response<ListItemsReply>, Status> {
    let items = service.get_items().execute(GetItemsQuery).await.map_err(map_error)?;
    Ok(Response::new(mapper::list_reply(&items)))
}
