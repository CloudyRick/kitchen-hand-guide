use actix_multipart::Multipart;
use actix_web::web;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use futures_util::StreamExt;
use sanitize_filename::sanitize;
use std::fs;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

/// Save an uploaded file and return the file path
pub async fn save_uploaded_file(
    mut payload: Multipart,
    upload_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Ensure upload directory exists
    fs::create_dir_all(upload_dir)?;

    while let Some(item) = payload.next().await {
        let mut field = item?;

        // Get the content disposition to extract filename
        let content_disposition = field.content_disposition();

        if let Some(filename) = content_disposition.get_filename() {
            // Sanitize the filename
            let sanitized_name = sanitize(filename);

            // Generate a unique filename using UUID
            let extension = Path::new(&sanitized_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("jpg");

            let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
            let filepath = Path::new(upload_dir).join(&unique_filename);

            // Create file
            let mut f = web::block(move || std::fs::File::create(filepath))
                .await??;

            // Write file chunks
            while let Some(chunk) = field.next().await {
                let data = chunk?;
                f = web::block(move || {
                    f.write_all(&data)?;
                    Ok::<_, std::io::Error>(f)
                })
                .await??;
            }

            // Return the URL path (relative to static directory)
            return Ok(format!("/static/uploads/{}", unique_filename));
        }
    }

    Err("No file found in upload".into())
}

/// Validate image file extension
pub fn is_valid_image_extension(filename: &str) -> bool {
    let valid_extensions = ["jpg", "jpeg", "png", "webp"];

    if let Some(ext) = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
    {
        valid_extensions.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

/// Get file size from multipart field
pub fn check_file_size(size: usize, max_size: usize) -> Result<(), String> {
    if size > max_size {
        return Err(format!(
            "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
            size, max_size
        ));
    }
    Ok(())
}

/// Initialize AWS S3 client
pub async fn init_s3_client() -> S3Client {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    S3Client::new(&config)
}

/// Upload file to S3 and return the public URL
pub async fn upload_to_s3(
    s3_client: &S3Client,
    bucket_name: &str,
    file_data: Bytes,
    filename: &str,
    content_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Generate unique filename with UUID
    let extension = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");

    let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
    let key = format!("uploads/{}", unique_filename);

    // Upload to S3
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key(&key)
        .body(ByteStream::from(file_data))
        .content_type(content_type)
        .send()
        .await?;

    // Return the public URL
    // Format: https://<bucket>.s3.<region>.amazonaws.com/<key>
    // Get region from environment or use default
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "ap-southeast-2".to_string());
    let url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket_name, region, key);
    Ok(url)
}

/// Get content type from filename extension
pub fn get_content_type(filename: &str) -> &'static str {
    let extension = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}
