use actix_web::dev::ServerHandle;
use alleycat_rs::startup;
use reqwest::Client;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashSet, net::TcpListener};

struct DocumentationApp {
    address: String,
    client: Client,
    server: ServerHandle,
}

impl DocumentationApp {
    async fn start() -> Self {
        // Exercise production registration without requiring a database. Closing
        // the lazy pool also prevents accidental database access in these tests.
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
            .unwrap();
        pool.close().await;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server = startup::run(listener, pool).unwrap();
        let handle = server.handle();
        tokio::spawn(server);
        Self {
            address,
            client: Client::new(),
            server: handle,
        }
    }

    async fn get(&self, path: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{path}", self.address))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
    }

    async fn spec(&self) -> Value {
        let response = self.get("/openapi.json").await;
        assert_eq!(response.headers()["content-type"], "application/json");
        response.json().await.unwrap()
    }
}

impl Drop for DocumentationApp {
    fn drop(&mut self) {
        tokio::spawn(self.server.stop(false));
    }
}

fn check_references(value: &Value, document: &Value) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref") {
                let reference = reference.as_str().unwrap();
                let pointer = reference.strip_prefix('#').expect("self-contained spec");
                assert!(
                    document.pointer(pointer).is_some(),
                    "unresolved {reference}"
                );
            }
            for child in object.values() {
                check_references(child, document);
            }
        }
        Value::Array(array) => {
            for child in array {
                check_references(child, document);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn registered_openapi_operations_are_internally_consistent() {
    let app = DocumentationApp::start().await;
    let spec = app.spec().await;
    assert_eq!(spec["info"]["title"], "Alleycat API");
    assert_eq!(spec["info"]["version"], env!("CARGO_PKG_VERSION"));
    check_references(&spec, &spec);
    let mut operation_ids = HashSet::new();
    let paths = spec["paths"].as_object().unwrap();
    assert!(!paths.is_empty());
    for (path, item) in paths {
        for method in [
            "get", "post", "put", "patch", "delete", "head", "options", "trace",
        ] {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let id = operation["operationId"].as_str().expect("operation ID");
            assert!(
                !id.is_empty() && operation_ids.insert(id),
                "duplicate/empty operation ID: {id}"
            );
            let responses = operation["responses"].as_object().unwrap();
            assert!(!responses.is_empty(), "{method} {path}");
            if let Some(no_content) = responses.get("204") {
                assert!(
                    no_content.get("content").is_none(),
                    "204 cannot have a body: {id}"
                );
            }
            let parameters: Vec<_> = item["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(operation["parameters"].as_array().into_iter().flatten())
                .filter(|p| p["in"] == "path")
                .collect();
            let placeholders: HashSet<_> = path
                .split('/')
                .filter_map(|segment| segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
                .collect();
            let names: HashSet<_> = parameters
                .iter()
                .map(|p| {
                    assert_eq!(p["required"], true, "{id}");
                    p["name"].as_str().unwrap()
                })
                .collect();
            assert_eq!(names, placeholders, "path parameters for {id}");
        }
    }
}

#[tokio::test]
async fn important_wire_contracts_survive_production_registration() {
    let app = DocumentationApp::start().await;
    let spec = app.spec().await;
    let paths = &spec["paths"];

    // Distinct wire formats and PATCH null semantics merit targeted assertions;
    // this is deliberately not an inventory of every handler or response field.
    let form = &paths["/players"]["post"]["requestBody"]["content"];
    assert!(form.get("application/x-www-form-urlencoded").is_some());
    assert!(form.get("application/json").is_none());
    let protobuf = &paths["/device-logs"]["post"]["requestBody"]["content"];
    assert_eq!(
        protobuf["application/protobuf"]["schema"],
        json!({"type": "string", "format": "binary"})
    );
    let update =
        &paths["/events/{id}"]["patch"]["requestBody"]["content"]["application/json"]["schema"];
    let update = spec
        .pointer(update["$ref"].as_str().unwrap().strip_prefix('#').unwrap())
        .unwrap();
    assert_eq!(update["properties"]["name"]["type"], "string");
    assert_eq!(
        update["properties"]["venue_name"]["type"],
        json!(["string", "null"])
    );
    assert!(
        update
            .get("required")
            .is_none_or(|required| required.as_array().unwrap().is_empty())
    );

    // Representative shared pagination consumer: no exhaustive call-site list.
    let players = &paths["/players"]["get"];
    for name in ["page", "per_page"] {
        let parameter = players["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        assert_eq!(parameter["in"], "query");
        assert_eq!(parameter["required"], false);
    }
    let failure = &players["responses"]["500"]["content"]["text/plain"]["schema"];
    assert_eq!(failure["type"], "string");

    // One actual response ties the published document to the running router.
    let health = app.get("/health_check").await;
    assert!(
        paths["/health_check"]["get"]["responses"]
            .get(health.status().as_str())
            .is_some()
    );
    assert!(health.bytes().await.unwrap().is_empty());
}

#[tokio::test]
async fn redoc_serves_local_assets_and_system_fonts() {
    let app = DocumentationApp::start().await;
    let page = app.get("/docs").await;
    assert_eq!(page.headers()["content-type"], "text/html; charset=utf-8");
    let html = page.text().await.unwrap();
    assert!(html.contains("src=\"/docs/redoc.js\""));
    assert!(html.contains("/openapi.json"));
    assert!(html.contains("system-ui, sans-serif"));
    assert!(html.contains("ui-monospace, monospace"));
    for external in [
        "https://",
        "http://",
        "src=\"//",
        "href=\"//",
        "@import",
        "@font-face",
    ] {
        assert!(!html.contains(external), "external dependency: {external}");
    }
    for (path, content_type) in [
        ("/docs/redoc.js", "text/javascript; charset=utf-8"),
        ("/docs/redoc-logo.svg", "image/svg+xml"),
    ] {
        let response = app.get(path).await;
        assert_eq!(response.headers()["content-type"], content_type);
        assert!(!response.bytes().await.unwrap().is_empty());
    }
}
