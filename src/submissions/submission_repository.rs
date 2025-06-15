use sqlx::PgPool;
use uuid::Uuid;
use serde_json::Value;

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
                request_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            submission_id,
            submission_type,
            session_id,
            user_id,
            status,
            submission_data as _,
            request_data as _
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
}
