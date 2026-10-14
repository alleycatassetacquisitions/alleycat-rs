use actix_web::{HttpMessage, HttpRequest, error};
use prost::Message;

// Describe the wire payload as binary, rather than a JSON array of byte values.
pub(super) struct ProtobufBody;

impl utoipa::PartialSchema for ProtobufBody {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{KnownFormat, ObjectBuilder, SchemaFormat, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Binary)))
            .into()
    }
}

impl utoipa::ToSchema for ProtobufBody {}

pub(super) fn decode<M: Message + Default>(
    request: &HttpRequest,
    body: &[u8],
) -> actix_web::Result<M> {
    if !request
        .content_type()
        .eq_ignore_ascii_case("application/protobuf")
    {
        return Err(error::ErrorUnsupportedMediaType(
            "Expected application/protobuf",
        ));
    }
    M::decode(body).map_err(|_| error::ErrorBadRequest("Invalid protobuf body"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::alleycat::device::WriteDeviceLogRequest;
    use actix_web::{http::header::CONTENT_TYPE, test::TestRequest};

    #[test]
    fn protobuf_decoding_checks_media_type_and_wire_format() {
        for value in ["application/protobuf", "Application/Protobuf; version=1"] {
            let request = TestRequest::default()
                .insert_header((CONTENT_TYPE, value))
                .to_http_request();
            assert!(decode::<WriteDeviceLogRequest>(&request, &[]).is_ok());
            let error = decode::<WriteDeviceLogRequest>(&request, &[0xff]).unwrap_err();
            assert_eq!(error.as_response_error().status_code(), 400);
        }
        for value in [
            None,
            Some("application/json"),
            Some("application/protobuf-extra"),
        ] {
            let mut request = TestRequest::default();
            if let Some(value) = value {
                request = request.insert_header((CONTENT_TYPE, value));
            }
            let error =
                decode::<WriteDeviceLogRequest>(&request.to_http_request(), &[]).unwrap_err();
            assert_eq!(error.as_response_error().status_code(), 415);
        }
    }
}
