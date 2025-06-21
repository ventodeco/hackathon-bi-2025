use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    commons::minio_service::MinioService,
    models::user::{ApiResponse, ApiError},
    services::{metrics_service::MetricsService, face_match_service::FaceMatchService},
    submissions::{
        submission_repository::SubmissionRepository,
        submission_service::SubmissionService,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedUrlsBody {
    pub submission_type: SubmissionType,
    pub nfc_identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceMatchBody {
    pub image1_url: String,
    pub image2_url: String,
    pub submission_id: String,
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

#[actix_web::post("/submissions/urls")]
async fn presigned_urls(
    pool: web::Data<sqlx::PgPool>,
    minio_service: web::Data<MinioService>,
    metrics: web::Data<MetricsService>,
    body: Result<web::Json<PresignedUrlsBody>, actix_web::Error>,
) -> HttpResponse {
    let body = match body {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                errors: Some(vec![ApiError {
                    entity: "HACKATHON_BI_2025".to_string(),
                    code: "1003".to_string(),
                    cause: format!("INVALID_REQUEST_BODY: {}", e),
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
            body.submission_type.clone(),
            body.nfc_identifier.clone(),
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

#[actix_web::post("/submissions/face-match")]
async fn face_match(
    face_match_service: web::Data<FaceMatchService>,
    body: Result<web::Json<FaceMatchBody>, actix_web::Error>,
) -> HttpResponse {
    let body = match body {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                errors: Some(vec![ApiError {
                    entity: "HACKATHON_BI_2025".to_string(),
                    code: "1003".to_string(),
                    cause: format!("INVALID_REQUEST_BODY: {}", e),
                }]),
            });
        }
    };

    match face_match_service
        .compare_faces(
            body.image1_url.clone(),
            body.image2_url.clone(),
            body.submission_id.clone(),
        )
        .await
    {
        Ok(response) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            errors: Some(vec![ApiError {
                entity: "HACKATHON_BI_2025".to_string(),
                code: "1006".to_string(),
                cause: e.to_string(),
            }]),
        }),
    }
}
