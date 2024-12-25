use actix_multipart::Multipart;
use reqwest::{Client, header::{HeaderMap, CONTENT_TYPE, AUTHORIZATION}};
use crate::signature::generate_auth_header;
use crate::utils::get_current_utc_date;
use log::{info, error};
use std::{env, collections::HashMap};

pub async fn upload_to_s3(filename: &str, file_data: Vec<u8>, bucket_name: &str, region: &str) -> Result<(), String> {
    let access_key = env::var("AWS_ACCESS_KEY").unwrap_or_else(|_| "your-access-key".to_string());
    let secret_key = env::var("AWS_SECRET_KEY").unwrap_or_else(|_| "your-secret-key".to_string());

    let date = get_current_utc_date(); // Get the current UTC date for signature
    let service = "s3"; // Service name for AWS S3

    let signature_key = crate::signature::get_signature_key(&secret_key, &date, &region, &service);

    let url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket_name, region, filename);

    // Create the headers for the request
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    
    let auth_header = generate_auth_header(&access_key, &signature_key, &date, &region, filename);
    headers.insert(AUTHORIZATION, reqwest::header::HeaderValue::from_str(&auth_header).unwrap());

    let client = Client::new();
    
    // Send the PUT request to S3
    match client.put(url)
        .headers(headers)
        .body(file_data)
        .send() {
            Ok(response) => {
                if response.status().is_success() {
                    info!("File uploaded to S3: {}", filename);
                    Ok(())
                } else {
                    let err_msg = format!("Failed to upload file: {}, Response: {}", filename, response.status());
                    error!("{}", err_msg);
                    Err(err_msg)
                }
            }
            Err(e) => {
                error!("Error uploading file to S3: {}. Error: {}", filename, e);
                Err(format!("Error uploading file to S3: {}", filename))
            }
        }
}

pub async fn save_file(mut payload: Multipart) -> impl Responder {
    let mut file_count = 0;
    let bucket_name = env::var("BUCKET_NAME").unwrap_or_else(|_| "your-bucket-name".to_string());
    let region = env::var("AWS_REGION").unwrap_or_else(|_| "your-region".to_string());

    // Iterate through multipart data (file parts)
    while let Some(item) = payload.next().await {
        let mut field = item.unwrap();
        let filename = field
            .content_disposition()
            .get_filename()
            .unwrap_or("unnamed")
            .to_string();

        // Skip the file if it's a .DS_Store file
        if filename == ".DS_Store" {
            info!("Skipping .DS_Store file.");
            continue; // Skip processing this file
        }

        let mut file_data = Vec::new();
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            file_data.extend_from_slice(&data);
        }

        // Upload the file to AWS S3
        match upload_to_s3(&filename, file_data, &bucket_name, &region).await {
            Ok(_) => {
                file_count += 1;
                info!("Successfully uploaded file: {}", filename);
            }
            Err(err) => {
                error!("{}", err);
            }
        }
    }

    info!("Uploaded {} files to S3.", file_count);
    HttpResponse::Ok().json(format!("{} files uploaded to S3", file_count))
}
