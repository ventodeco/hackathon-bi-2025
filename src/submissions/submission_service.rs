use std::{collections::HashMap, time::Duration};
use uuid::Uuid;
use serde_json::json;
use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::{
    commons::minio_service::{self, MinioService},
    models::user::ApiError,
    services::metrics_service::MetricsService,
    submissions::{
        dto::presigned_urls_response::{Document, PresignedUrlsResponse, SubmissionData}, submission_controller::SubmissionType, submission_repository::SubmissionRepository
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
        submission_type: SubmissionType,
        nfc_identifier: String,
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
        let ktp_filename = ktp_uuid.to_string() + "_KTP";
        let ktp_url = match self.minio_service
            .generate_upload_url(ktp_filename.clone(), Duration::from_secs(600))
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
        let selfie_filename = selfie_uuid.to_string() + "_SELFIE";
        let selfie_url = match self.minio_service
            .generate_upload_url(selfie_filename.clone(), Duration::from_secs(600))
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

        // TODO: nfc_identifier is base64 of the image, i want to upload to minio
        let nfc_identifier_clean = nfc_identifier.replace("data:image/jpeg;base64,", "").replace("data:image/png;base64,", "");
        let nfc_identifier_base64 = STANDARD.decode(&nfc_identifier_clean).unwrap();
        let nfc_uuid = Uuid::new_v4();
        let nfc_identifier_filename = nfc_uuid.to_string() + "_NFC";
        self.minio_service.upload_file(nfc_identifier_filename.clone(), nfc_identifier_base64, Some("image/jpeg".to_string())).await.unwrap();

        let mut documents_data = HashMap::new();
        documents_data.insert("KTP", SubmissionData {
            document_name: ktp_filename.clone(),
            document_reference: ktp_uuid.to_string(),
        });
        documents_data.insert("SELFIE", SubmissionData {
            document_name: selfie_filename.clone(),
            document_reference: selfie_uuid.to_string()
        });
        documents_data.insert("NFC", SubmissionData {
            document_name: nfc_identifier_filename.clone(),
            document_reference: nfc_uuid.to_string(),
        });

        // Save to database
        if let Err(e) = self
            .submission_repository
            .create(
                submission_id,
                &format!("{:?}", submission_type),
                &session_id,
                &user_id,
                "INITAITED",
                json!(documents_data),
                json!({}),
                &nfc_identifier[..200],
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
