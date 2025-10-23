use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // Subject (user ID)
    pub username: String, // Username
    pub exp: usize,       // Expiration time
    pub iat: usize,       // Issued at
}

/// Hash a password using bcrypt
///
/// # Arguments
/// * `password` - Plain text password to hash
///
/// # Returns
/// Result containing the hashed password or an error
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify a password against a hash
///
/// # Arguments
/// * `password` - Plain text password to verify
/// * `hash` - Bcrypt hash to compare against
///
/// # Returns
/// Result containing boolean (true if password matches) or an error
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

/// Generate a JWT token for a user
///
/// # Arguments
/// * `user_id` - UUID of the user
/// * `username` - Username of the user
///
/// # Returns
/// Result containing the JWT token string or an error
pub fn generate_token(user_id: Uuid, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env file");
    let expiration_hours = env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse::<i64>()
        .unwrap_or(24);

    let now = Utc::now();
    let exp = (now + Duration::hours(expiration_hours))
        .timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
        iat,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

/// Validate and decode a JWT token
///
/// # Arguments
/// * `token` - JWT token string to validate
///
/// # Returns
/// Result containing the Claims if valid, or an error
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env file");

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

/// Extract the user ID from a JWT token
///
/// # Arguments
/// * `token` - JWT token string
///
/// # Returns
/// Result containing the user UUID if valid, or an error
pub fn get_user_id_from_token(token: &str) -> Result<Uuid, String> {
    match validate_token(token) {
        Ok(claims) => Uuid::parse_str(&claims.sub).map_err(|e| e.to_string()),
        Err(e) => Err(format!("Invalid token: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hashed = hash_password(password).expect("Failed to hash password");

        assert!(verify_password(password, &hashed).expect("Failed to verify password"));
        assert!(!verify_password("wrong_password", &hashed).expect("Failed to verify password"));
    }

    #[test]
    fn test_token_generation_and_validation() {
        // Set up environment variable for test
        env::set_var("JWT_SECRET", "test_secret_key_for_testing");
        env::set_var("JWT_EXPIRATION_HOURS", "24");

        let user_id = Uuid::new_v4();
        let username = "testuser";

        let token = generate_token(user_id, username).expect("Failed to generate token");

        let claims = validate_token(&token).expect("Failed to validate token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, username);
    }

    #[test]
    fn test_expired_token() {
        env::set_var("JWT_SECRET", "test_secret_key_for_testing");

        // Create a token that's already expired
        let user_id = Uuid::new_v4();
        let claims = Claims {
            sub: user_id.to_string(),
            username: "testuser".to_string(),
            exp: (Utc::now() - Duration::hours(1)).timestamp() as usize,
            iat: (Utc::now() - Duration::hours(2)).timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test_secret_key_for_testing"),
        )
        .expect("Failed to create expired token");

        // This should fail because the token is expired
        assert!(validate_token(&token).is_err());
    }
}
