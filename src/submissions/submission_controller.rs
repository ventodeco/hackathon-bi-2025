use actix_web::{web, HttpResponse, error::QueryPayloadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    commons::minio_service::MinioService,
    models::user::{ApiResponse, ApiError},
    services::metrics_service::MetricsService,
    submissions::{
        submission_repository::SubmissionRepository,
        submission_service::SubmissionService,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedUrlsQuery {
    pub submission_type: SubmissionType,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub enum SubmissionType {
    KYC
}

impl std::fmt::Display for SubmissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmissionType::KYC => write!(f, "KYC"),
        }
    }
}

#[actix_web::get("/submissions/urls")]
async fn presigned_urls(
    pool: web::Data<sqlx::PgPool>,
    minio_service: web::Data<MinioService>,
    metrics: web::Data<MetricsService>,
    query: Result<web::Query<PresignedUrlsQuery>, actix_web::Error>,
) -> HttpResponse {
    let query = match query {
        Ok(q) => q,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                errors: Some(vec![ApiError {
                    entity: "HACKATHON_BI_2025".to_string(),
                    code: "1003".to_string(),
                    cause: format!("INVALID_QUERY_PARAMETERS: {}", e),
                }]),
            });
        }
    };

    // TODO: Get these from auth middleware
    let session_id = Uuid::new_v4().to_string();
    let user_id = "1".to_string();

    let submission_service = SubmissionService::new(
        minio_service.as_ref().clone(),
        SubmissionRepository::new(pool.as_ref().clone()),
        metrics.get_ref().clone()
    );

    match submission_service
        .generate_presigned_urls(
            session_id,
            user_id,
            query.submission_type.clone(),
        )
        .await
    {
        Ok(response) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }),
        Err(errors) => HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            errors: Some(errors),
        }),
    }
}
