use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use tracing::{info, info_span};
use validator::Validate;

use crate::{
    models::user::{ApiError, ApiResponse, AuthResponse, LoginRequest, RegisterRequest},
    services::auth_service::AuthService,
};

#[actix_web::post("/register")]
async fn register(
    pool: web::Data<PgPool>,
    request: web::Json<RegisterRequest>,
) -> HttpResponse {
    // Validate request
    if let Err(_) = request.validate() {
        return HttpResponse::UnprocessableEntity().json(ApiResponse::<AuthResponse> {
            success: false,
            data: None,
            errors: Some(vec![ApiError {
                entity: "BACKEND_PROJECT".to_string(),
                code: "1001".to_string(),
                cause: "INVALID_EMAIL_OR_PASSWORD".to_string(),
            }]),
        });
    }

    // Get JWT secret from environment
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    // Create auth service
    let auth_service = AuthService::new(pool.get_ref().clone(), jwt_secret);

    // Handle registration
    match auth_service.register(request.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(response),
            errors: None,
        }),
        Err(e) => {
            if e.to_string() == "User already exists" {
                HttpResponse::UnprocessableEntity().json(ApiResponse::<AuthResponse> {
                    success: false,
                    data: None,
                    errors: Some(vec![ApiError {
                        entity: "BACKEND_PROJECT".to_string(),
                        code: "1002".to_string(),
                        cause: "USER_ALREADY_EXISTS".to_string(),
                    }]),
                })
            } else {
                HttpResponse::InternalServerError().json(ApiResponse::<AuthResponse> {
                    success: false,
                    data: None,
                    errors: Some(vec![ApiError {
                        entity: "BACKEND_PROJECT".to_string(),
                        code: "1000".to_string(),
                        cause: "SYSTEM_ERROR".to_string(),
                    }]),
                })
            }
        }
    }
}

#[actix_web::post("/login")]
async fn login(
    pool: web::Data<PgPool>,
    request: web::Json<LoginRequest>,
) -> HttpResponse {
    let _span = info_span!("login-api", correlation_id = uuid::Uuid::new_v4().to_string()).entered();
    let start = std::time::Instant::now();
    // Validate request
    if let Err(_) = request.validate() {
        return HttpResponse::UnprocessableEntity().json(ApiResponse::<AuthResponse> {
            success: false,
            data: None,
            errors: Some(vec![ApiError {
                entity: "BACKEND_PROJECT".to_string(),
                code: "1001".to_string(),
                cause: "INVALID_EMAIL_OR_PASSWORD".to_string(),
            }]),
        });
    }

    info!(test = "uhuy", uhuy = "aaa", "Validation process took: {:?}", start.elapsed());

    let duration = start.elapsed();
    info!("Validation process took: {:?}", duration);

    let start = std::time::Instant::now();
    // Get JWT secret from environment
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let duration = start.elapsed();
    info!("JWT secret process took: {:?}", duration);

    let start = std::time::Instant::now();
    // Create auth service
    let auth_service = AuthService::new(pool.get_ref().clone(), jwt_secret);

    let duration = start.elapsed();
    info!("Auth service process took: {:?}", duration);

    // Handle login
    let start = std::time::Instant::now();
    match auth_service.login(request.into_inner()).await {
        Ok(response) => {
            let duration = start.elapsed();
            info!("Login process took: {:?}", duration);
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some(response),
                errors: None,
            })
        },
        Err(e) => {
            if e.to_string() == "Invalid email or password" {
                HttpResponse::UnprocessableEntity().json(ApiResponse::<AuthResponse> {
                    success: false,
                    data: None,
                    errors: Some(vec![ApiError {
                        entity: "BACKEND_PROJECT".to_string(),
                        code: "1001".to_string(),
                        cause: "INVALID_EMAIL_OR_PASSWORD".to_string(),
                    }]),
                })
            } else {
                HttpResponse::InternalServerError().json(ApiResponse::<AuthResponse> {
                    success: false,
                    data: None,
                    errors: Some(vec![ApiError {
                        entity: "BACKEND_PROJECT".to_string(),
                        code: "1000".to_string(),
                        cause: "SYSTEM_ERROR".to_string(),
                    }]),
                })
            }
        }
    }
} 