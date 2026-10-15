use actix_web::{HttpResponse, web};
use utoipa::openapi::{Info, OpenApi};
use utoipa_redoc::Redoc;

pub fn configure_documentation(config: &mut web::ServiceConfig, mut api: OpenApi) {
    api.info = Info::new("Alleycat API", env!("CARGO_PKG_VERSION"));
    api.info.description = Some(
        "HTTP API for the Alleycat Rust server. Player operations use the active event. \
         These routes currently do not enforce authentication.\n\n\
         [Device APIs](/docs/device-apis) — protobuf message fields, types, and validation requirements."
            .into(),
    );
    config
        .route("/", web::get().to(api_index))
        .route("/docs", web::get().to(redoc_page))
        .route("/docs/device-apis", web::get().to(device_apis))
        .route("/docs/device-apis.css", web::get().to(device_apis_css))
        .route("/docs/device_api.proto", web::get().to(device_proto))
        .route("/docs/redoc.js", web::get().to(redoc_script))
        .route("/docs/redoc-logo.svg", web::get().to(redoc_logo))
        .route("/openapi.json", web::get().to(openapi))
        .app_data(web::Data::new(api));
}

async fn api_index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/index.html"))
}

async fn device_apis() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/device-apis/index.html"))
}

async fn device_apis_css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(include_str!("../../static/device-apis/style.css"))
}

async fn device_proto() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(include_str!("../../proto/device_api.proto"))
}

async fn redoc_page() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Redoc::new("/openapi.json")
                .custom_html(include_str!("../../static/redoc/index.html"))
                .to_html(),
        )
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
