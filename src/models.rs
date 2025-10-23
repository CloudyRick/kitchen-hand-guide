use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for Product
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub supplier_name: String,
    pub product_name: String,
    pub location: String,
    pub picture_url: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Form data for creating a new product
#[derive(Debug, Deserialize)]
pub struct NewProductForm {
    pub supplier_name: String,
    pub product_name: String,
    pub location: String,
    pub description: String,
}

impl NewProductForm {
    /// Validate the form data
    pub fn validate(&self) -> Result<(), String> {
        if self.supplier_name.trim().is_empty() {
            return Err("Supplier name cannot be empty".to_string());
        }
        if self.product_name.trim().is_empty() {
            return Err("Product name cannot be empty".to_string());
        }
        if self.location.trim().is_empty() {
            return Err("Location cannot be empty".to_string());
        }
        if self.description.trim().is_empty() {
            return Err("Description cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Database operations for Product
impl Product {
    /// Get all products from database
    pub async fn get_all(pool: &sqlx::PgPool) -> Result<Vec<Product>, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "SELECT id, supplier_name, product_name, location, picture_url, description, created_at, updated_at
             FROM products
             ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
    }

    /// Get a single product by ID
    pub async fn get_by_id(pool: &sqlx::PgPool, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "SELECT id, supplier_name, product_name, location, picture_url, description, created_at, updated_at
             FROM products
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Create a new product
    pub async fn create(
        pool: &sqlx::PgPool,
        supplier_name: &str,
        product_name: &str,
        location: &str,
        picture_url: &str,
        description: &str,
    ) -> Result<Product, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "INSERT INTO products (supplier_name, product_name, location, picture_url, description)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, supplier_name, product_name, location, picture_url, description, created_at, updated_at"
        )
        .bind(supplier_name)
        .bind(product_name)
        .bind(location)
        .bind(picture_url)
        .bind(description)
        .fetch_one(pool)
        .await
    }

    /// Update an existing product
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        supplier_name: &str,
        product_name: &str,
        location: &str,
        picture_url: &str,
        description: &str,
    ) -> Result<Product, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "UPDATE products
             SET supplier_name = $2, product_name = $3, location = $4, picture_url = $5, description = $6, updated_at = CURRENT_TIMESTAMP
             WHERE id = $1
             RETURNING id, supplier_name, product_name, location, picture_url, description, created_at, updated_at"
        )
        .bind(id)
        .bind(supplier_name)
        .bind(product_name)
        .bind(location)
        .bind(picture_url)
        .bind(description)
        .fetch_one(pool)
        .await
    }
}

/// Database model for Preparation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Preparation {
    pub id: Uuid,
    pub name: String,
    pub prep_type: String,
    pub shift: String,
    pub location: String,
    pub picture_url: String,
    pub steps: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Form data for creating a new preparation
#[derive(Debug, Deserialize)]
pub struct NewPreparationForm {
    pub name: String,
    pub prep_type: String,
    pub shift: String,
    pub location: String,
    pub steps: String,
}

impl NewPreparationForm {
    /// Validate the form data
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Preparation name cannot be empty".to_string());
        }
        if !["fruit", "bread", "veg", "meat", "seafood"].contains(&self.prep_type.as_str()) {
            return Err("Invalid preparation type".to_string());
        }
        if !["brekkie", "lunch", "both"].contains(&self.shift.as_str()) {
            return Err("Invalid shift selection".to_string());
        }
        if self.location.trim().is_empty() {
            return Err("Location cannot be empty".to_string());
        }
        if self.steps.trim().is_empty() {
            return Err("Steps cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Database operations for Preparation
impl Preparation {
    /// Get all preparations from database
    pub async fn get_all(pool: &sqlx::PgPool) -> Result<Vec<Preparation>, sqlx::Error> {
        sqlx::query_as::<_, Preparation>(
            "SELECT id, name, prep_type, shift, location, picture_url, steps, created_at, updated_at
             FROM preparations
             ORDER BY prep_type, name"
        )
        .fetch_all(pool)
        .await
    }

    /// Get a single preparation by ID
    pub async fn get_by_id(pool: &sqlx::PgPool, id: Uuid) -> Result<Option<Preparation>, sqlx::Error> {
        sqlx::query_as::<_, Preparation>(
            "SELECT id, name, prep_type, shift, location, picture_url, steps, created_at, updated_at
             FROM preparations
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Create a new preparation
    pub async fn create(
        pool: &sqlx::PgPool,
        name: &str,
        prep_type: &str,
        shift: &str,
        location: &str,
        picture_url: &str,
        steps: &str,
    ) -> Result<Preparation, sqlx::Error> {
        sqlx::query_as::<_, Preparation>(
            "INSERT INTO preparations (name, prep_type, shift, location, picture_url, steps)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, name, prep_type, shift, location, picture_url, steps, created_at, updated_at"
        )
        .bind(name)
        .bind(prep_type)
        .bind(shift)
        .bind(location)
        .bind(picture_url)
        .bind(steps)
        .fetch_one(pool)
        .await
    }

    /// Update an existing preparation
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: &str,
        prep_type: &str,
        shift: &str,
        location: &str,
        picture_url: &str,
        steps: &str,
    ) -> Result<Preparation, sqlx::Error> {
        sqlx::query_as::<_, Preparation>(
            "UPDATE preparations
             SET name = $2, prep_type = $3, shift = $4, location = $5, picture_url = $6, steps = $7, updated_at = CURRENT_TIMESTAMP
             WHERE id = $1
             RETURNING id, name, prep_type, shift, location, picture_url, steps, created_at, updated_at"
        )
        .bind(id)
        .bind(name)
        .bind(prep_type)
        .bind(shift)
        .bind(location)
        .bind(picture_url)
        .bind(steps)
        .fetch_one(pool)
        .await
    }
}

/// Database model for PreparationStep
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PreparationStep {
    pub id: Uuid,
    pub preparation_id: Uuid,
    pub step_number: i32,
    pub description: String,
    pub picture_url: String,
    pub created_at: DateTime<Utc>,
}

/// Database operations for PreparationStep
impl PreparationStep {
    /// Get all steps for a preparation
    pub async fn get_by_preparation_id(
        pool: &sqlx::PgPool,
        preparation_id: Uuid,
    ) -> Result<Vec<PreparationStep>, sqlx::Error> {
        sqlx::query_as::<_, PreparationStep>(
            "SELECT id, preparation_id, step_number, description, picture_url, created_at
             FROM preparation_steps
             WHERE preparation_id = $1
             ORDER BY step_number ASC"
        )
        .bind(preparation_id)
        .fetch_all(pool)
        .await
    }

    /// Create a new preparation step
    pub async fn create(
        pool: &sqlx::PgPool,
        preparation_id: Uuid,
        step_number: i32,
        description: &str,
        picture_url: &str,
    ) -> Result<PreparationStep, sqlx::Error> {
        sqlx::query_as::<_, PreparationStep>(
            "INSERT INTO preparation_steps (preparation_id, step_number, description, picture_url)
             VALUES ($1, $2, $3, $4)
             RETURNING id, preparation_id, step_number, description, picture_url, created_at"
        )
        .bind(preparation_id)
        .bind(step_number)
        .bind(description)
        .bind(picture_url)
        .fetch_one(pool)
        .await
    }

    /// Delete all steps for a preparation
    pub async fn delete_by_preparation_id(
        pool: &sqlx::PgPool,
        preparation_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM preparation_steps WHERE preparation_id = $1")
            .bind(preparation_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Database model for User
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Form data for user login
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// Form data for user registration
#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

impl RegisterForm {
    /// Validate the registration form
    pub fn validate(&self) -> Result<(), String> {
        if self.username.trim().is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if self.username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        if self.username.len() > 50 {
            return Err("Username cannot exceed 50 characters".to_string());
        }
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }
        if self.email.trim().is_empty() {
            return Err("Email cannot be empty".to_string());
        }
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err("Invalid email format".to_string());
        }
        if self.password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }
        if self.password != self.confirm_password {
            return Err("Passwords do not match".to_string());
        }
        Ok(())
    }
}

/// Database operations for User
impl User {
    /// Get a user by username
    pub async fn get_by_username(
        pool: &sqlx::PgPool,
        username: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, is_active, created_at, updated_at
             FROM users
             WHERE username = $1 AND is_active = true"
        )
        .bind(username)
        .fetch_optional(pool)
        .await
    }

    /// Get a user by email
    pub async fn get_by_email(
        pool: &sqlx::PgPool,
        email: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, is_active, created_at, updated_at
             FROM users
             WHERE email = $1 AND is_active = true"
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    /// Get a user by ID
    pub async fn get_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, is_active, created_at, updated_at
             FROM users
             WHERE id = $1 AND is_active = true"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Create a new user
    pub async fn create(
        pool: &sqlx::PgPool,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash)
             VALUES ($1, $2, $3)
             RETURNING id, username, email, password_hash, is_active, created_at, updated_at"
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await
    }
}
