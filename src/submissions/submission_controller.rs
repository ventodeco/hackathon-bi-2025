use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::{
    commons::minio_service::MinioService,
    models::user::ApiResponse,
    services::metrics_service::MetricsService,
    submissions::{
        submission_repository::SubmissionRepository,
        submission_service::SubmissionService,
    },
};

#[actix_web::get("/submissions/urls")]
async fn presigned_urls(
    pool: web::Data<sqlx::PgPool>,
    minio_service: web::Data<MinioService>,
    metrics: web::Data<MetricsService>,
) -> HttpResponse {
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
