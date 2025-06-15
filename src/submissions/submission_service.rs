use std::{collections::HashMap, time::Duration};
use uuid::Uuid;
use serde_json::json;

use crate::{
    commons::minio_service::MinioService,
    models::user::ApiError,
    services::metrics_service::MetricsService,
    submissions::{
        dto::presigned_urls_response::{Document, PresignedUrlsResponse},
        submission_repository::SubmissionRepository,
    },
};

pub struct SubmissionService {
    minio_service: MinioService,
    submission_repository: SubmissionRepository,
    metrics: MetricsService,
}

impl SubmissionService {
    pub fn new(
        minio_service: MinioService, 
        submission_repository: SubmissionRepository, 
        metrics: MetricsService
    ) -> Self {
        Self {
            minio_service,
            submission_repository,
            metrics,
        }
    }

    pub async fn generate_presigned_urls(
        &self,
        session_id: String,
        user_id: String,
    ) -> Result<PresignedUrlsResponse, Vec<ApiError>> {
        let start = std::time::Instant::now();
        let mut tags = HashMap::new();
        tags.insert("endpoint".to_string(), "presigned_urls".to_string());

        // Generate a new submission ID
        let submission_id = Uuid::new_v4();

        // Generate document references and presigned URLs
        let mut documents = HashMap::new();

        // KTP document
        let ktp_uuid = Uuid::new_v4();
        let ktp_url = match self.minio_service
            .generate_upload_url(ktp_uuid.to_string() + "_KTP", Duration::from_secs(600))
            .await
        {
            Ok(url) => url,
            Err(e) => {
                self.metrics.increment("api_error", Some(tags.clone()));
                return Err(vec![ApiError {
                    entity: "HACKATHON_BI_2025".to_string(),
                    code: "1001".to_string(),
                    cause: e.to_string(),
                }]);
            }
        };

        documents.insert(
            "KTP".to_string(),
            Document {
                document_url: ktp_url,
                document_reference: ktp_uuid.to_string(),
                expiry_in_seconds: "600".to_string(),
            },
        );

        // Selfie document
        let selfie_uuid = Uuid::new_v4();
        let selfie_url = match self.minio_service
            .generate_upload_url(selfie_uuid.to_string() + "_SELFIE", Duration::from_secs(600))
            .await
        {
            Ok(url) => url,
            Err(e) => {
                self.metrics.increment("api_error", Some(tags.clone()));
                return Err(vec![ApiError {
                    entity: "HACKATHON_BI_2025".to_string(),
                    code: "1001".to_string(),
                    cause: e.to_string(),
                }]);
            }
        };

        documents.insert(
            "SELFIE".to_string(),
            Document {
                document_url: selfie_url,
                document_reference: selfie_uuid.to_string(),
                expiry_in_seconds: "600".to_string(),
            },
        );

        let response = PresignedUrlsResponse {
            submission_id: submission_id.to_string(),
            documents,
        };

        // Save to database
        if let Err(e) = self
            .submission_repository
            .create(
                submission_id,
                "DOCUMENT_UPLOAD",
                &session_id,
                &user_id,
                "PENDING",
                json!(response),
                json!({}),
            )
            .await
        {
            self.metrics.increment("api_error", Some(tags.clone()));
            return Err(vec![ApiError {
                entity: "HACKATHON_BI_2025".to_string(),
                code: "1002".to_string(),
                cause: e.to_string(),
            }]);
        }

        self.metrics.increment("api_success", Some(tags.clone()));
        self.metrics.timing("api_latency", start.elapsed(), Some(tags));

        Ok(response)
    }
}