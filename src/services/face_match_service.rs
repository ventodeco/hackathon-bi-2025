use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::time::Duration;

use crate::services::metrics_service::MetricsService;

#[derive(Debug, Serialize)]
pub struct FaceMatchRequest {
    pub image1_url: String,
    pub image2_url: String,
    pub submission_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FaceMatchResponse {
    pub submission_id: String,
    pub similarity_score: f64,
    pub is_match: bool,
    pub threshold: f64,
}

#[derive(Clone)]
pub struct FaceMatchService {
    client: reqwest::Client,
    base_url: String,
    threshold: f64,
    metrics: MetricsService,
}

impl FaceMatchService {
    pub fn new(
        base_url: String,
        threshold: f64,
        timeout_millis: u64,
        metrics: MetricsService,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_millis))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            threshold,
            metrics,
        }
    }

    pub async fn compare_faces(
        &self,
        image1_url: String,
        image2_url: String,
        submission_id: String,
    ) -> Result<FaceMatchResponse> {
        let start = std::time::Instant::now();
        let mut tags = HashMap::new();
        tags.insert("endpoint".to_string(), "face_match".to_string());

        let url = format!(
            "{}/compare-faces?image1_url={}&image2_url={}",
            self.base_url, image1_url, image2_url
        );

        let response = match self
            .client
            .post(&url)
            .header("x-submission-id", &submission_id)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.metrics.increment("face_match.error", Some(tags.clone()));
                self.metrics.timing("face_match.duration", start.elapsed(), Some(tags));
                return Err(anyhow::anyhow!("HTTP request failed: {}", e));
            }
        };

        if !response.status().is_success() {
            self.metrics.increment("face_match.error", Some(tags.clone()));
            self.metrics.timing("face_match.duration", start.elapsed(), Some(tags));
            return Err(anyhow::anyhow!(
                "Face match API returned error status: {}",
                response.status()
            ));
        }

        let face_match_response: FaceMatchResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                self.metrics.increment("face_match.error", Some(tags.clone()));
                self.metrics.timing("face_match.duration", start.elapsed(), Some(tags));
                return Err(anyhow::anyhow!("Failed to parse response: {}", e));
            }
        };

        // Check if the match meets our threshold
        let is_above_threshold = face_match_response.similarity_score >= self.threshold;
        
        if is_above_threshold {
            self.metrics.increment("face_match.success", Some(tags.clone()));
        } else {
            self.metrics.increment("face_match.failure", Some(tags.clone()));
        }

        self.metrics.timing("face_match.duration", start.elapsed(), Some(tags));

        Ok(face_match_response)
    }

    pub fn get_threshold(&self) -> f64 {
        self.threshold
    }
} 