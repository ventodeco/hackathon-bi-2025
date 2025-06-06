use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    models::user::{AuthResponse, LoginRequest, RegisterRequest, User},
    repositories::user_repository::UserRepository,
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32,
    exp: i64,
}

pub struct AuthService {
    user_repository: UserRepository,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt_secret: String) -> Self {
        Self {
            user_repository: UserRepository::new(pool),
            jwt_secret,
        }
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, anyhow::Error> {
        // Check if user exists
        if let Some(_) = self.user_repository.find_by_email(&request.email).await? {
            return Err(anyhow::anyhow!("User already exists"));
        }

        // Hash password
        let password_hash = hash(request.password.as_bytes(), DEFAULT_COST)?;

        // Create user
        let user = self
            .user_repository
            .create(&request.name, &request.email, &password_hash)
            .await?;

        // Generate token
        self.generate_token(user.id)
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, anyhow::Error> {
        // Find user
        let user = self
            .user_repository
            .find_by_email(&request.email)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;

        // Verify password
        if !verify(&request.password, &user.password_hash)? {
            return Err(anyhow::anyhow!("Invalid email or password"));
        }

        // Generate token
        self.generate_token(user.id)
    }

    fn generate_token(&self, user_id: i32) -> Result<AuthResponse, anyhow::Error> {
        let expiration = Utc::now() + Duration::hours(24);
        let claims = Claims {
            sub: user_id,
            exp: expiration.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(AuthResponse {
            token,
            expired_at: expiration,
        })
    }
} 