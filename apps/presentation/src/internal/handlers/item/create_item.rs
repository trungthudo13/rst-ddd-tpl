use application::CreateItemCommand;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::internal::{
    error::map_error,
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
    let command = CreateItemCommand {
        title: request.into_inner().title,
    };
    service
        .create_item()
        .execute(command)
        .await
        .map(ItemReply::from)
        .map(Response::new)
        .map_err(map_error)
}
