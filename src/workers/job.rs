use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadJob {
    pub id: Uuid,
    pub esign_id: String,
    pub document_url: String,
    pub document_name: String,
    pub document_type: String,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    UrlExpired,
    DeadLetter,
}

impl FileUploadJob {
    pub fn new(
        esign_id: String,
        document_url: String,
        document_name: String,
        document_type: String,
        metadata: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            esign_id,
            document_url,
            document_name,
            document_type,
            retry_count: 0,
            created_at: now,
            updated_at: now,
            metadata,
        }
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn get_lock_key(&self) -> String {
        format!("upload_lock:{}", self.esign_id)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
