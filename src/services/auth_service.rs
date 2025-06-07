use argon2::{self, password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    models::user::{AuthResponse, LoginRequest, RegisterRequest},
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
        let start = std::time::Instant::now();
        // Check if user exists
        if let Some(_) = self.user_repository.find_by_email(&request.email).await? {
            return Err(anyhow::anyhow!("User already exists"));
        }

        let duration = start.elapsed();
        log::info!("User check process took: {:?}", duration);

        let start = std::time::Instant::now();
        // Hash password with Argon2
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let argon2 = argon2::Argon2::default();
        let password_hash = PasswordHasher::hash_password(
            &argon2,
            request.password.as_bytes(),
            &salt,
        ).map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

        let duration = start.elapsed();
        log::info!("Password hash process took: {:?}", duration);

        let start = std::time::Instant::now();
        // Create user
        let user = self
            .user_repository
            .create(&request.name, &request.email, &password_hash.to_string())
            .await?;

        let duration = start.elapsed();
        log::info!("User creation process took: {:?}", duration);

        // Generate token
        self.generate_token(user.id)
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, anyhow::Error> {
        let start = std::time::Instant::now();
        // Find user
        let user = self
            .user_repository
            .find_by_email(&request.email)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;

        let duration = start.elapsed();
        log::info!("User find process took: {:?}", duration);

        let start = std::time::Instant::now();
        // Verify password with Argon2
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;
        let argon2 = argon2::Argon2::default();
        if PasswordVerifier::verify_password(&argon2, request.password.as_bytes(), &parsed_hash).is_err() {
            return Err(anyhow::anyhow!("Invalid email or password"));
        }

        let duration = start.elapsed();
        log::info!("Password verify process took: {:?}", duration);

        // Generate token
        self.generate_token(user.id)
    }

    fn generate_token(&self, user_id: i32) -> Result<AuthResponse, anyhow::Error> {
        let start = std::time::Instant::now();
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

        let duration = start.elapsed();
        log::info!("Token generate process took: {:?}", duration);

        Ok(AuthResponse {
            token,
            expired_at: expiration,
        })
    }
} 