use actix_web::{web, HttpResponse};
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, info_span};
use std::collections::HashMap;

use crate::{
    models::user::{ApiError, ApiResponse, AuthResponse, LoginRequest, RegisterRequest},
    services::{auth_service::AuthService, metrics_service::MetricsService},
};

#[derive(Debug, Serialize)]
struct Document {
    document_url: String,
    document_reference: String,
    expiry_in_seconds: String,
}

#[derive(Debug, Serialize)]
struct PresignedUrlsResponse {
    submission_id: String,
    documents: HashMap<String, Document>,
}

#[actix_web::get("/submissions/urls")]
async fn presigned_urls(
    pool: web::Data<PgPool>,
    metrics: web::Data<MetricsService>,
) -> HttpResponse {
    let start = std::time::Instant::now();
    let mut tags = HashMap::new();
    tags.insert("endpoint".to_string(), "presigned_urls".to_string());

    let response: PresignedUrlsResponse = PresignedUrlsResponse {
        submission_id: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
        documents: HashMap::from([
            ("KTP".to_string(), Document {
                document_url: "https://example.com/presigned-url1".to_string(),
                document_reference: "4gb96g75-6828-5673-b3fc-2c963f66afa6".to_string(),
                expiry_in_seconds: "600".to_string(),
            }),
        ]),
    };

    metrics.increment("submissions.presigned_urls.success", Some(tags.clone()));
    metrics.timing("submissions.presigned_urls.duration", start.elapsed(), Some(tags));

    return HttpResponse::Ok().json(
        ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }
    );
}
