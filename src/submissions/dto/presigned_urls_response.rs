use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub document_url: String,
    pub document_reference: String,
    pub expiry_in_seconds: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedUrlsResponse {
    pub submission_id: String,
    pub documents: HashMap<String, Document>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionData {
    pub document_name: String,
    pub document_reference: String,
}