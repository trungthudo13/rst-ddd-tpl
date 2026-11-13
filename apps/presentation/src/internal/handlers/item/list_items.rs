use application::GetItemsQuery;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::internal::{
    error::map_error,
    pb::item::v1::{
        ItemReply,
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
    Ok(Response::new(ListItemsReply {
        items: items.into_iter().map(ItemReply::from).collect(),
    }))
}
