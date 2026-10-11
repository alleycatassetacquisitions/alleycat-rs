use actix_web::error;

// Handle only unexpected database failures here. Expected conflicts and missing
// resources belong to the route. Messages must be fixed, safe client-facing text.
pub(super) fn database_error(error: sqlx::Error, message: &'static str) -> actix_web::Error {
    // Log the details in the current request span, never in the response.
    tracing::error!(?error, "{message}");
    error::ErrorInternalServerError(message)
}
