use sqlx::PgPool;
use uuid::Uuid;
use serde_json::{Value, json};

pub struct SubmissionRepository {
    pool: PgPool,
}

impl SubmissionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        submission_id: Uuid,
        submission_type: &str,
        session_id: &str,
        user_id: &str,
        status: &str,
        submission_data: Value,
        request_data: Value,
        nfc_identifier: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO submissions (
                submission_id,
                submission_type,
                session_id,
                user_id,
                status,
                submission_data,
                request_data,
                nfc_identifier
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            submission_id,
            submission_type,
            session_id,
            user_id,
            status,
            submission_data as _,
            request_data as _,
            nfc_identifier
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_submission_id(&self, submission_id: Uuid) -> Result<Option<Value>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT submission_data
            FROM submissions
            WHERE submission_id = $1
            "#,
            submission_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|r| r.submission_data.and_then(|s| serde_json::from_str(&s).ok())))
    }

    pub async fn find_submission_by_id(&self, submission_id: &str) -> Result<Option<(String, Value)>, sqlx::Error> {
        let submission_uuid = Uuid::parse_str(submission_id).map_err(|_| sqlx::Error::RowNotFound)?;
        
        let result = sqlx::query!(
            r#"
            SELECT status, submission_data
            FROM submissions
            WHERE submission_id = $1
            "#,
            submission_uuid
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| {
            let status = r.status;
            let data = r.submission_data
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(json!({}));
            (status, data)
        }))
    }

    pub async fn update_submission_status(&self, submission_id: &str, status: &str) -> Result<(), sqlx::Error> {
        let submission_uuid = Uuid::parse_str(submission_id).map_err(|_| sqlx::Error::RowNotFound)?;
        
        sqlx::query!(
            r#"
            UPDATE submissions
            SET status = $2, updated_at = NOW()
            WHERE submission_id = $1
            "#,
            submission_uuid,
            status
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_submission_by_nfc_identifier_and_submission_type(&self, submission_type: &str, nfc_identifier: &str) -> Result<Option<String>, sqlx::Error> {
        
        let result = sqlx::query!(
            r#"
            SELECT status
            FROM submissions
            WHERE submission_type = $1 AND nfc_identifier = $2
            order by id desc limit 1
            "#,
            submission_type,
            nfc_identifier
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.status))
    }
}
