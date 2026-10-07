use actix_web::HttpResponse;
use actix_web::get;

#[utoipa::path(
    tag = "System", summary = "Check server availability",
    responses((status = 200, description = "Server is running; empty body. This does not check database health."))
)]
#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
