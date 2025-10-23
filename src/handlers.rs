use crate::auth;
use crate::models::{LoginForm, NewPreparationForm, NewProductForm, Preparation, PreparationStep, Product, RegisterForm, User};
use crate::utils;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Result};
use askama::Template;
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::io::Read;
use uuid::Uuid;

/// Template for the index page
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    products: Vec<Product>,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the new product form
#[derive(Template)]
#[template(path = "product_new.html")]
struct ProductNewTemplate {
    error: String,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the product detail page
#[derive(Template)]
#[template(path = "product_detail.html")]
struct ProductDetailTemplate {
    product: Product,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the product edit page
#[derive(Template)]
#[template(path = "product_edit.html")]
struct ProductEditTemplate {
    product: Product,
    error: String,
    is_authenticated: bool,
    username: Option<String>,
}

/// Multipart form structure for file upload
#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(limit = "20 MB")]
    picture: Option<TempFile>,
    supplier_name: Text<String>,
    product_name: Text<String>,
    location: Text<String>,
    description: Text<String>,
}

/// Multipart form structure for preparation upload
#[derive(Debug, MultipartForm)]
pub struct PreparationUploadForm {
    #[multipart(limit = "20 MB")]
    picture: Option<TempFile>,
    name: Text<String>,
    prep_type: Text<String>,
    shift: Text<String>,
    location: Text<String>,
    steps: Text<String>,
}

/// GET / - Homepage with list of products
pub async fn index(
    pool: web::Data<sqlx::PgPool>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let products = Product::get_all(pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch products")
        })?;

    let template = IndexTemplate {
        products,
        is_authenticated: auth.user.is_some(),
        username: auth.user.map(|u| u.username),
    };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /product/new - Show form to add new product
pub async fn new_product_form(
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let template = ProductNewTemplate {
        error: String::new(),
        is_authenticated: auth.user.is_some(),
        username: auth.user.map(|u| u.username),
    };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /product - Handle form submission and insert into DB
pub async fn create_product(
    pool: web::Data<sqlx::PgPool>,
    s3_client: web::Data<S3Client>,
    auth: crate::middleware::OptionalAuth,
    MultipartForm(form): MultipartForm<UploadForm>,
) -> Result<HttpResponse> {
    // Validate form data
    let form_data = NewProductForm {
        supplier_name: form.supplier_name.to_string(),
        product_name: form.product_name.to_string(),
        location: form.location.to_string(),
        description: form.description.to_string(),
    };

    if let Err(error_msg) = form_data.validate() {
        let template = ProductNewTemplate {
            error: error_msg,
            is_authenticated: auth.user.is_some(),
            username: auth.user.map(|u| u.username),
        };
        let html = template.render().map_err(|e| {
            eprintln!("Template error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;
        return Ok(HttpResponse::BadRequest()
            .content_type("text/html")
            .body(html));
    }

    // Handle optional image upload
    let picture_url = if let Some(picture) = form.picture {
        // Validate file extension
        let filename = picture.file_name.as_ref().ok_or_else(|| {
            actix_web::error::ErrorBadRequest("Invalid file uploaded")
        })?;

        if !utils::is_valid_image_extension(filename) {
            let template = ProductNewTemplate {
                error: "Invalid file type. Only JPG, PNG, and WEBP are allowed.".to_string(),
                is_authenticated: auth.user.is_some(),
                username: auth.user.map(|u| u.username),
            };
            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;
            return Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(html));
        }

        // Read file data
        let file_path = picture.file.path();
        let mut file_content = Vec::new();
        let mut file = std::fs::File::open(file_path).map_err(|e| {
            eprintln!("Failed to open uploaded file: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to read uploaded file")
        })?;
        file.read_to_end(&mut file_content).map_err(|e| {
            eprintln!("Failed to read uploaded file: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to read uploaded file")
        })?;

        // Check if S3 is enabled
        let s3_enabled = std::env::var("S3_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if s3_enabled {
            // Upload to S3
            let bucket_name = std::env::var("S3_BUCKET_NAME")
                .unwrap_or_else(|_| "kitchen-hand-guide".to_string());
            let content_type = utils::get_content_type(filename);

            utils::upload_to_s3(
                s3_client.get_ref(),
                &bucket_name,
                Bytes::from(file_content),
                filename,
                content_type,
            )
            .await
            .map_err(|e| {
                eprintln!("S3 upload error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to upload file to S3")
            })?
        } else {
            // Save to local filesystem (fallback)
            let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./static/uploads".to_string());
            std::fs::create_dir_all(&upload_dir).map_err(|e| {
                eprintln!("Failed to create upload directory: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to create upload directory")
            })?;

            let extension = std::path::Path::new(filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("jpg");

            let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
            let filepath = std::path::Path::new(&upload_dir).join(&unique_filename);

            picture.file.persist(&filepath).map_err(|e| {
                eprintln!("Failed to save uploaded file: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to save uploaded file")
            })?;

            format!("/static/uploads/{}", unique_filename)
        }
    } else {
        // No image provided, use empty string
        String::new()
    };

    // Insert into database
    let product = Product::create(
        pool.get_ref(),
        &form_data.supplier_name,
        &form_data.product_name,
        &form_data.location,
        &picture_url,
        &form_data.description,
    )
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to create product")
    })?;

    // Redirect to the newly created product's detail page
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/product/{}", product.id)))
        .finish())
}

/// GET /product/{id} - View details of a single product
pub async fn product_detail(
    pool: web::Data<sqlx::PgPool>,
    id: web::Path<Uuid>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let product = Product::get_by_id(pool.get_ref(), *id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch product")
        })?;

    match product {
        Some(product) => {
            let template = ProductDetailTemplate {
                product,
                is_authenticated: auth.user.is_some(),
                username: auth.user.map(|u| u.username),
            };

            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        None => Ok(HttpResponse::NotFound()
            .content_type("text/html")
            .body("<h1>404 - Product Not Found</h1><p><a href='/'>Back to Home</a></p>")),
    }
}

// ============== PREPARATION HANDLERS ==============

/// Template for the preparations index page
#[derive(Template)]
#[template(path = "preparations_index.html")]
struct PreparationsIndexTemplate {
    preparations: Vec<Preparation>,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the new preparation form
#[derive(Template)]
#[template(path = "preparation_new.html")]
struct PreparationNewTemplate {
    error: String,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the preparation detail page
#[derive(Template)]
#[template(path = "preparation_detail.html")]
struct PreparationDetailTemplate {
    preparation: Preparation,
    steps: Vec<PreparationStep>,
    is_authenticated: bool,
    username: Option<String>,
}

/// Template for the preparation edit page
#[derive(Template)]
#[template(path = "preparation_edit.html")]
struct PreparationEditTemplate {
    preparation: Preparation,
    steps: Vec<PreparationStep>,
    error: String,
    is_authenticated: bool,
    username: Option<String>,
}

/// GET /preparations - List all preparations
pub async fn preparations_index(
    pool: web::Data<sqlx::PgPool>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let preparations = Preparation::get_all(pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch preparations")
        })?;

    let template = PreparationsIndexTemplate {
        preparations,
        is_authenticated: auth.user.is_some(),
        username: auth.user.map(|u| u.username),
    };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// GET /preparation/new - Show form to add new preparation
pub async fn new_preparation_form(
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let template = PreparationNewTemplate {
        error: String::new(),
        is_authenticated: auth.user.is_some(),
        username: auth.user.map(|u| u.username),
    };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /preparation - Handle form submission and insert into DB
pub async fn create_preparation(
    pool: web::Data<sqlx::PgPool>,
    s3_client: web::Data<S3Client>,
    auth: crate::middleware::OptionalAuth,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    let mut name = String::new();
    let mut prep_type = String::new();
    let mut shift = String::new();
    let mut location = String::new();
    let mut steps_text = String::new();
    let mut picture_url = String::new();

    // HashMap to store step descriptions and images
    // Key: step number, Value: (description, optional image data)
    let mut steps_data: HashMap<usize, (String, Option<(Vec<u8>, String)>)> = HashMap::new();

    // Process multipart form
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            eprintln!("Multipart error: {:?}", e);
            actix_web::error::ErrorBadRequest("Invalid multipart data")
        })?;

        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");

        if field_name == "name" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            name = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "prep_type" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            prep_type = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "shift" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            shift = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "location" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            location = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "steps" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            steps_text = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "picture" {
            // Main preparation image (optional)
            if let Some(filename) = content_disposition.get_filename().map(|s| s.to_string()) {
                if utils::is_valid_image_extension(&filename) {
                    let mut file_data = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading file: {}", e)))?;
                        file_data.extend_from_slice(&data);
                    }
                    picture_url = upload_image_to_storage(&s3_client, &file_data, &filename).await?;
                }
            }
        } else if field_name.starts_with("step_description_") {
            // Extract step number from field name
            if let Some(num_str) = field_name.strip_prefix("step_description_") {
                if let Ok(step_num) = num_str.parse::<usize>() {
                    let mut bytes = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                        bytes.extend_from_slice(&data);
                    }
                    let description = String::from_utf8_lossy(&bytes).to_string();
                    steps_data.entry(step_num).or_insert((String::new(), None)).0 = description;
                }
            }
        } else if field_name.starts_with("step_image_") {
            // Extract step number from field name
            if let Some(num_str) = field_name.strip_prefix("step_image_") {
                if let Ok(step_num) = num_str.parse::<usize>() {
                    if let Some(filename) = content_disposition.get_filename().map(|s| s.to_string()) {
                        if utils::is_valid_image_extension(&filename) && !filename.is_empty() {
                            let mut file_data = Vec::new();
                            while let Some(chunk) = field.next().await {
                                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading file: {}", e)))?;
                                file_data.extend_from_slice(&data);
                            }
                            if !file_data.is_empty() {
                                steps_data.entry(step_num).or_insert((String::new(), None)).1 = Some((file_data, filename));
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate form data
    let form_data = NewPreparationForm {
        name: name.clone(),
        prep_type: prep_type.clone(),
        shift: shift.clone(),
        location: location.clone(),
        steps: steps_text.clone(),
    };

    if let Err(error_msg) = form_data.validate() {
        let template = PreparationNewTemplate {
            error: error_msg,
            is_authenticated: auth.user.is_some(),
            username: auth.user.map(|u| u.username),
        };
        let html = template.render().map_err(|e| {
            eprintln!("Template error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;
        return Ok(HttpResponse::BadRequest()
            .content_type("text/html")
            .body(html));
    }

    // Create preparation
    let preparation = Preparation::create(
        pool.get_ref(),
        &name,
        &prep_type,
        &shift,
        &location,
        &picture_url,
        &steps_text,
    )
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to create preparation")
    })?;

    // Create preparation steps
    let mut sorted_steps: Vec<_> = steps_data.into_iter().collect();
    sorted_steps.sort_by_key(|(num, _)| *num);

    for (idx, (_step_num, (description, image_data))) in sorted_steps.iter().enumerate() {
        let step_picture_url = if let Some((data, filename)) = image_data {
            upload_image_to_storage(&s3_client, data, filename).await?
        } else {
            String::new()
        };

        PreparationStep::create(
            pool.get_ref(),
            preparation.id,
            (idx + 1) as i32,  // Use sequential numbering
            description,
            &step_picture_url,
        )
        .await
        .map_err(|e| {
            eprintln!("Database error creating step: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to create preparation step")
        })?;
    }

    // Redirect to the newly created preparation's detail page
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/preparation/{}", preparation.id)))
        .finish())
}

/// Helper function to upload an image to S3 or local storage
async fn upload_image_to_storage(
    s3_client: &web::Data<S3Client>,
    file_data: &[u8],
    filename: &str,
) -> Result<String> {
    let s3_enabled = std::env::var("S3_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if s3_enabled {
        // Upload to S3
        let bucket_name = std::env::var("S3_BUCKET_NAME")
            .unwrap_or_else(|_| "kitchen-hand-guide".to_string());
        let content_type = utils::get_content_type(filename);

        utils::upload_to_s3(
            s3_client.get_ref(),
            &bucket_name,
            Bytes::from(file_data.to_vec()),
            filename,
            content_type,
        )
        .await
        .map_err(|e| {
            eprintln!("S3 upload error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to upload file to S3")
        })
    } else {
        // Save to local filesystem (fallback)
        let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./static/uploads".to_string());
        std::fs::create_dir_all(&upload_dir).map_err(|e| {
            eprintln!("Failed to create upload directory: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to create upload directory")
        })?;

        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let filepath = std::path::Path::new(&upload_dir).join(&unique_filename);

        std::fs::write(&filepath, file_data).map_err(|e| {
            eprintln!("Failed to save uploaded file: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to save uploaded file")
        })?;

        Ok(format!("/static/uploads/{}", unique_filename))
    }
}

/// GET /preparation/{id} - View details of a single preparation
pub async fn preparation_detail(
    pool: web::Data<sqlx::PgPool>,
    preparation_id: web::Path<Uuid>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let preparation = Preparation::get_by_id(pool.get_ref(), *preparation_id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch preparation")
        })?;

    match preparation {
        Some(preparation) => {
            // Fetch steps for this preparation
            let steps = PreparationStep::get_by_preparation_id(pool.get_ref(), *preparation_id)
                .await
                .map_err(|e| {
                    eprintln!("Database error fetching steps: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to fetch preparation steps")
                })?;

            let template = PreparationDetailTemplate {
                preparation,
                steps,
                is_authenticated: auth.user.is_some(),
                username: auth.user.map(|u| u.username),
            };

            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        None => Ok(HttpResponse::NotFound()
            .content_type("text/html")
            .body("<h1>404 - Preparation Not Found</h1><p><a href='/preparations'>Back to Preparations</a></p>")),
    }
}

// ============== EDIT HANDLERS ==============

/// GET /product/{id}/edit - Show edit form for a product
pub async fn edit_product_form(
    pool: web::Data<sqlx::PgPool>,
    id: web::Path<Uuid>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let product = Product::get_by_id(pool.get_ref(), *id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch product")
        })?;

    match product {
        Some(product) => {
            let template = ProductEditTemplate {
                product,
                error: String::new(),
                is_authenticated: auth.user.is_some(),
                username: auth.user.map(|u| u.username),
            };

            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        None => Ok(HttpResponse::NotFound()
            .content_type("text/html")
            .body("<h1>404 - Product Not Found</h1><p><a href='/'>Back to Home</a></p>")),
    }
}

/// POST /product/{id} - Update an existing product
pub async fn update_product(
    pool: web::Data<sqlx::PgPool>,
    s3_client: web::Data<S3Client>,
    id: web::Path<Uuid>,
    auth: crate::middleware::OptionalAuth,
    MultipartForm(form): MultipartForm<UploadForm>,
) -> Result<HttpResponse> {
    // Fetch existing product
    let existing_product = Product::get_by_id(pool.get_ref(), *id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch product")
        })?;

    let existing_product = match existing_product {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body("<h1>404 - Product Not Found</h1>"));
        }
    };

    // Validate form data
    let form_data = NewProductForm {
        supplier_name: form.supplier_name.to_string(),
        product_name: form.product_name.to_string(),
        location: form.location.to_string(),
        description: form.description.to_string(),
    };

    if let Err(error_msg) = form_data.validate() {
        let template = ProductEditTemplate {
            product: existing_product,
            error: error_msg,
            is_authenticated: auth.user.is_some(),
            username: auth.user.map(|u| u.username),
        };
        let html = template.render().map_err(|e| {
            eprintln!("Template error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;
        return Ok(HttpResponse::BadRequest()
            .content_type("text/html")
            .body(html));
    }

    // Check if new image was uploaded
    let picture_url = if let Some(picture) = &form.picture {
        if let Some(filename) = &picture.file_name {
            if !filename.is_empty() && utils::is_valid_image_extension(filename) {
                // Read and upload new image
                let file_path = picture.file.path();
                let mut file_content = Vec::new();
                let mut file = std::fs::File::open(file_path).map_err(|e| {
                    eprintln!("Failed to open uploaded file: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to read uploaded file")
                })?;
                file.read_to_end(&mut file_content).map_err(|e| {
                    eprintln!("Failed to read uploaded file: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to read uploaded file")
                })?;

                upload_image_to_storage(&s3_client, &file_content, filename).await?
            } else {
                // Keep existing image
                existing_product.picture_url.clone()
            }
        } else {
            // Keep existing image
            existing_product.picture_url.clone()
        }
    } else {
        // Keep existing image
        existing_product.picture_url.clone()
    };

    // Update product
    let product = Product::update(
        pool.get_ref(),
        *id,
        &form_data.supplier_name,
        &form_data.product_name,
        &form_data.location,
        &picture_url,
        &form_data.description,
    )
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to update product")
    })?;

    // Redirect to product detail page
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/product/{}", product.id)))
        .finish())
}

/// GET /preparation/{id}/edit - Show edit form for a preparation
pub async fn edit_preparation_form(
    pool: web::Data<sqlx::PgPool>,
    preparation_id: web::Path<Uuid>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let preparation = Preparation::get_by_id(pool.get_ref(), *preparation_id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch preparation")
        })?;

    match preparation {
        Some(preparation) => {
            let steps = PreparationStep::get_by_preparation_id(pool.get_ref(), *preparation_id)
                .await
                .map_err(|e| {
                    eprintln!("Database error fetching steps: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to fetch preparation steps")
                })?;

            let template = PreparationEditTemplate {
                preparation,
                steps,
                error: String::new(),
                is_authenticated: auth.user.is_some(),
                username: auth.user.map(|u| u.username),
            };

            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        None => Ok(HttpResponse::NotFound()
            .content_type("text/html")
            .body("<h1>404 - Preparation Not Found</h1><p><a href='/preparations'>Back to Preparations</a></p>")),
    }
}

/// POST /preparation/{id} - Update an existing preparation
pub async fn update_preparation(
    pool: web::Data<sqlx::PgPool>,
    s3_client: web::Data<S3Client>,
    preparation_id: web::Path<Uuid>,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    // Fetch existing preparation
    let existing_prep = Preparation::get_by_id(pool.get_ref(), *preparation_id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch preparation")
        })?;

    let existing_prep = match existing_prep {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body("<h1>404 - Preparation Not Found</h1>"));
        }
    };

    let mut name = String::new();
    let mut prep_type = String::new();
    let mut shift = String::new();
    let mut location = String::new();
    let mut steps_text = String::new();
    let mut picture_url = existing_prep.picture_url.clone();
    let mut steps_data: HashMap<usize, (String, Option<(Vec<u8>, String)>)> = HashMap::new();

    // Process multipart form (same as create_preparation)
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            eprintln!("Multipart error: {:?}", e);
            actix_web::error::ErrorBadRequest("Invalid multipart data")
        })?;

        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");

        if field_name == "name" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            name = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "prep_type" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            prep_type = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "shift" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            shift = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "location" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            location = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "steps" {
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            steps_text = String::from_utf8_lossy(&bytes).to_string();
        } else if field_name == "picture" {
            if let Some(filename) = content_disposition.get_filename().map(|s| s.to_string()) {
                if utils::is_valid_image_extension(&filename) && !filename.is_empty() {
                    let mut file_data = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading file: {}", e)))?;
                        file_data.extend_from_slice(&data);
                    }
                    if !file_data.is_empty() {
                        picture_url = upload_image_to_storage(&s3_client, &file_data, &filename).await?;
                    }
                }
            }
        } else if field_name.starts_with("step_description_") {
            if let Some(num_str) = field_name.strip_prefix("step_description_") {
                if let Ok(step_num) = num_str.parse::<usize>() {
                    let mut bytes = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading field: {}", e)))?;
                        bytes.extend_from_slice(&data);
                    }
                    let description = String::from_utf8_lossy(&bytes).to_string();
                    steps_data.entry(step_num).or_insert((String::new(), None)).0 = description;
                }
            }
        } else if field_name.starts_with("step_image_") {
            if let Some(num_str) = field_name.strip_prefix("step_image_") {
                if let Ok(step_num) = num_str.parse::<usize>() {
                    if let Some(filename) = content_disposition.get_filename().map(|s| s.to_string()) {
                        if utils::is_valid_image_extension(&filename) && !filename.is_empty() {
                            let mut file_data = Vec::new();
                            while let Some(chunk) = field.next().await {
                                let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Error reading file: {}", e)))?;
                                file_data.extend_from_slice(&data);
                            }
                            if !file_data.is_empty() {
                                steps_data.entry(step_num).or_insert((String::new(), None)).1 = Some((file_data, filename));
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate
    let form_data = NewPreparationForm {
        name: name.clone(),
        prep_type: prep_type.clone(),
        shift: shift.clone(),
        location: location.clone(),
        steps: steps_text.clone(),
    };

    if let Err(error_msg) = form_data.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("text/html")
            .body(format!("<h1>Validation Error</h1><p>{}</p><a href='/preparation/{}/edit'>Go Back</a>", error_msg, preparation_id)));
    }

    // Update preparation
    Preparation::update(
        pool.get_ref(),
        *preparation_id,
        &name,
        &prep_type,
        &shift,
        &location,
        &picture_url,
        &steps_text,
    )
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to update preparation")
    })?;

    // Delete existing steps
    PreparationStep::delete_by_preparation_id(pool.get_ref(), *preparation_id)
        .await
        .map_err(|e| {
            eprintln!("Database error deleting steps: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to delete old steps")
        })?;

    // Create new steps
    let mut sorted_steps: Vec<_> = steps_data.into_iter().collect();
    sorted_steps.sort_by_key(|(num, _)| *num);

    for (idx, (_step_num, (description, image_data))) in sorted_steps.iter().enumerate() {
        let step_picture_url = if let Some((data, filename)) = image_data {
            upload_image_to_storage(&s3_client, data, filename).await?
        } else {
            String::new()
        };

        PreparationStep::create(
            pool.get_ref(),
            *preparation_id,
            (idx + 1) as i32,
            description,
            &step_picture_url,
        )
        .await
        .map_err(|e| {
            eprintln!("Database error creating step: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to create preparation step")
        })?;
    }

    // Redirect to preparation detail page
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/preparation/{}", preparation_id)))
        .finish())
}

// ============== SEARCH HANDLER ==============

/// Template for the search results page
#[derive(Template)]
#[template(path = "search_results.html")]
struct SearchResultsTemplate {
    query: String,
    products: Vec<Product>,
    preparations: Vec<Preparation>,
    is_authenticated: bool,
    username: Option<String>,
}

/// Query parameters for search
#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    q: String,
}

/// GET /search - Search for products and preparations
pub async fn search(
    pool: web::Data<sqlx::PgPool>,
    query: web::Query<SearchQuery>,
    auth: crate::middleware::OptionalAuth,
) -> Result<HttpResponse> {
    let search_term = query.q.trim();

    // Search products - using ILIKE for case-insensitive search
    let products = sqlx::query_as::<_, Product>(
        "SELECT id, supplier_name, product_name, location, picture_url, description, created_at, updated_at
         FROM products
         WHERE product_name ILIKE $1
            OR supplier_name ILIKE $1
            OR location ILIKE $1
            OR description ILIKE $1
         ORDER BY product_name"
    )
    .bind(format!("%{}%", search_term))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Database error searching products: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to search products")
    })?;

    // Search preparations - using ILIKE for case-insensitive search
    let preparations = sqlx::query_as::<_, Preparation>(
        "SELECT id, name, prep_type, shift, location, picture_url, steps, created_at, updated_at
         FROM preparations
         WHERE name ILIKE $1
            OR prep_type ILIKE $1
            OR shift ILIKE $1
            OR location ILIKE $1
            OR steps ILIKE $1
         ORDER BY name"
    )
    .bind(format!("%{}%", search_term))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Database error searching preparations: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to search preparations")
    })?;

    let template = SearchResultsTemplate {
        query: search_term.to_string(),
        products,
        preparations,
        is_authenticated: auth.user.is_some(),
        username: auth.user.map(|u| u.username),
    };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ============== AUTHENTICATION HANDLERS ==============

/// Template for login page
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: String,
}

/// Template for register page
#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    error: String,
}

/// GET /login - Show login form
pub async fn login_form(auth: crate::middleware::OptionalAuth) -> Result<HttpResponse> {
    // If already logged in, redirect to home
    if auth.user.is_some() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/"))
            .finish());
    }

    let template = LoginTemplate { error: String::new() };

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /login - Handle login submission
pub async fn login(
    pool: web::Data<sqlx::PgPool>,
    form: web::Form<LoginForm>,
) -> Result<HttpResponse> {
    // Find user by username
    let user = User::get_by_username(pool.get_ref(), &form.username)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

    match user {
        Some(user) => {
            // Verify password
            match auth::verify_password(&form.password, &user.password_hash) {
                Ok(true) => {
                    // Password correct - generate JWT token
                    let token = auth::generate_token(user.id, &user.username)
                        .map_err(|e| {
                            eprintln!("Token generation error: {:?}", e);
                            actix_web::error::ErrorInternalServerError("Failed to generate token")
                        })?;

                    // Set cookie and redirect to home
                    Ok(HttpResponse::SeeOther()
                        .append_header(("Location", "/"))
                        .cookie(
                            actix_web::cookie::Cookie::build("auth_token", token)
                                .path("/")
                                .http_only(true)
                                .finish()
                        )
                        .finish())
                }
                Ok(false) => {
                    // Password incorrect
                    let template = LoginTemplate {
                        error: "Invalid username or password".to_string(),
                    };
                    let html = template.render().map_err(|e| {
                        eprintln!("Template error: {:?}", e);
                        actix_web::error::ErrorInternalServerError("Failed to render template")
                    })?;
                    Ok(HttpResponse::Unauthorized()
                        .content_type("text/html")
                        .body(html))
                }
                Err(e) => {
                    eprintln!("Password verification error: {:?}", e);
                    Err(actix_web::error::ErrorInternalServerError("Authentication error"))
                }
            }
        }
        None => {
            // User not found
            let template = LoginTemplate {
                error: "Invalid username or password".to_string(),
            };
            let html = template.render().map_err(|e| {
                eprintln!("Template error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to render template")
            })?;
            Ok(HttpResponse::Unauthorized()
                .content_type("text/html")
                .body(html))
        }
    }
}

/// GET /register - Show registration form (TEMPORARILY DISABLED)
pub async fn register_form() -> Result<HttpResponse> {
    // Registration temporarily disabled
    Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body("<h1>Registration Temporarily Disabled</h1><p>Please contact an administrator for access.</p><p><a href='/login'>Go to Login</a> | <a href='/'>Go to Home</a></p>"))
}

/// POST /register - Handle registration submission (TEMPORARILY DISABLED)
pub async fn register(
    _pool: web::Data<sqlx::PgPool>,
    _form: web::Form<RegisterForm>,
) -> Result<HttpResponse> {
    // Registration temporarily disabled
    Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body("<h1>Registration Temporarily Disabled</h1><p>Please contact an administrator for access.</p><p><a href='/login'>Go to Login</a> | <a href='/'>Go to Home</a></p>"))
}

/// GET /logout - Handle logout
pub async fn logout() -> Result<HttpResponse> {
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/"))
        .cookie(
            actix_web::cookie::Cookie::build("auth_token", "")
                .path("/")
                .http_only(true)
                .max_age(actix_web::cookie::time::Duration::seconds(0))
                .finish()
        )
        .finish())
}

// ============== ERROR HANDLERS ==============

/// Template for 401 Unauthorized error page
#[derive(Template)]
#[template(path = "401.html")]
struct Error401Template {}

/// GET /401 - Show 401 Unauthorized page
pub async fn error_401() -> Result<HttpResponse> {
    let template = Error401Template {};

    let html = template.render().map_err(|e| {
        eprintln!("Template error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Failed to render template")
    })?;

    Ok(HttpResponse::Unauthorized()
        .content_type("text/html")
        .body(html))
}
