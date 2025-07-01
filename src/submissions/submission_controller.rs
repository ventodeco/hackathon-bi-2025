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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSubmissionBody {
    pub submission_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubmissionStatusQuery {
    pub submission_type: String,
    pub nfc_identifier: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSubmissionResponse {
    pub submission_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubmissionStatusResponse {
    pub submission_status: String,
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

#[actix_web::put("/submissions/urls")]
async fn process_submission(
    pool: web::Data<sqlx::PgPool>,
    minio_service: web::Data<MinioService>,
    face_match_service: web::Data<FaceMatchService>,
    metrics: web::Data<MetricsService>,
    body: Result<web::Json<ProcessSubmissionBody>, actix_web::Error>,
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

    let submission_service = SubmissionService::new(
        minio_service.as_ref().clone(),
        SubmissionRepository::new(pool.as_ref().clone()),
        metrics.as_ref().clone()
    );

    match submission_service
        .process_submission(
            body.submission_id.clone(),
            face_match_service.as_ref().clone()
        )
        .await
    {
        Ok(response) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }),
        Err(errors) => {
            let status_code = if errors.iter().any(|e| e.code == "1004") {
                HttpResponse::UnprocessableEntity
            } else {
                HttpResponse::InternalServerError
            };
            
            status_code().json(ApiResponse::<()> {
                success: false,
                data: None,
                errors: Some(errors),
            })
        }
    }
}

#[actix_web::get("/submissions/status")]
async fn get_submission_status(
    pool: web::Data<sqlx::PgPool>,
    minio_service: web::Data<MinioService>,
    metrics: web::Data<MetricsService>,
    query: web::Query<GetSubmissionStatusQuery>,
) -> HttpResponse {

    let submission_type = match query.submission_type.as_str() {
        "KYC" => SubmissionType::KYC,
        _ => return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            errors: Some(vec![ApiError {
                entity: "HACKATHON_BI_2025".to_string(),
                code: "1003".to_string(),
                cause: "INVALID_SUBMISSION_TYPE".to_string(),
            }]),
        }),
    };

    let nfc_identifier = query.nfc_identifier.clone();

    let submission_service = SubmissionService::new(
        minio_service.as_ref().clone(),
        SubmissionRepository::new(pool.as_ref().clone()),
        metrics.as_ref().clone()
    );

    match submission_service.get_submission_status(submission_type, nfc_identifier).await {
        Ok(response) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }),
        Err(errors) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                data: None,
                errors: Some(errors),
            })
        }
    }
}
