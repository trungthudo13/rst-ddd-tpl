/// Report service health.
#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service is healthy")),
)]
pub async fn health() -> &'static str {
    "ok"
}
