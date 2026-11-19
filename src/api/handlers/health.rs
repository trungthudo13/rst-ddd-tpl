#[utoipa::path(
    tag = "health",
    description = "Check process liveness.",
    get,
    path = "/health",
    responses((status = 200, description = "Service is healthy")),
)]
pub async fn health() -> &'static str {
    "ok"
}
