use application::GetItemQuery;
use tonic::{
    Request,
    Response,
    Status,
};
use uuid::Uuid;

use crate::internal::{
    error::map_error,
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
    let id = Uuid::parse_str(&request.into_inner().id)
        .map_err(|_| Status::invalid_argument("invalid item id"))?;
    service
        .get_item()
        .execute(GetItemQuery {
            id,
        })
        .await
        .map(ItemReply::from)
        .map(Response::new)
        .map_err(map_error)
}
