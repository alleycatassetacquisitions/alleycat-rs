use actix_web::{HttpResponse, web};
use utoipa::openapi::{Info, OpenApi};
use utoipa_redoc::{Redoc, Servable};

pub fn configure_documentation(config: &mut web::ServiceConfig, mut api: OpenApi) {
    api.info = Info::new("Alleycat API", env!("CARGO_PKG_VERSION"));
    api.info.description = Some(
        "HTTP API for the Alleycat Rust server. Player operations use the active event. \
         These routes currently do not enforce authentication."
            .into(),
    );
    config
        .service(
            Redoc::with_url("/docs", "/openapi.json")
                .custom_html(include_str!("../../static/redoc/index.html")),
        )
        .route("/docs/redoc.js", web::get().to(redoc_script))
        .route("/docs/redoc-logo.svg", web::get().to(redoc_logo))
        .route("/openapi.json", web::get().to(openapi))
        .app_data(web::Data::new(api));
}

async fn redoc_script() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/javascript; charset=utf-8")
        .insert_header(("Cache-Control", "no-cache"))
        .body(&include_bytes!("../../static/redoc/redoc.standalone.js")[..])
}

async fn redoc_logo() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("../../static/redoc/logo-mini.svg"))
}

async fn openapi(api: web::Data<OpenApi>) -> web::Json<OpenApi> {
    web::Json(api.get_ref().clone())
}
