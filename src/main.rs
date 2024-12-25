use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_multipart::Multipart;
use futures_util::stream::StreamExt;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::env;
use log::{info, error};

async fn save_file(mut payload: Multipart) -> impl Responder {
    let mut file_count = 0;
    // Improved error handling for getting the current directory
    let temp_dir = match env::current_dir() {
        Ok(path) => path.join("uploads"),
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            return HttpResponse::InternalServerError().json("Failed to get current directory");
        }
    };

    // Ensure the root uploads directory exists
    if !temp_dir.exists() {
        match create_dir_all(&temp_dir) {
            Ok(_) => info!("Created uploads directory at {:?}", temp_dir),
            Err(e) => {
                error!("Failed to create uploads directory {:?}: {}", temp_dir, e);
                return HttpResponse::InternalServerError().json("Failed to create uploads directory");
            }
        }
    }

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

        // Check if the filename contains .DS_Store (for files in subdirectories)
        if filename.contains(".DS_Store") {
            info!("Skipping .DS_Store file in subdirectory: {}", filename);
            continue; // Skip this file
        }

        // Logging the filename and the path where it's being saved
        info!("Processing file: {} to {:?}", filename, temp_dir);

        let filepath = temp_dir.join(filename.clone()); // Clone filename to preserve it for later use

        // Create the necessary directories for the file path
        if let Some(parent_dir) = filepath.parent() {
            if !parent_dir.exists() {
                match create_dir_all(parent_dir) {
                    Ok(_) => info!("Created directory: {:?}", parent_dir),
                    Err(e) => {
                        error!("Failed to create directory {:?}: {}", parent_dir, e);
                        return HttpResponse::InternalServerError().json(format!("Failed to create directory: {:?}", parent_dir));
                    }
                }
            }
        }

        // Now we can safely create the file
        let mut f = match File::create(&filepath) {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to create file at {:?}: {}. Error: {}", filepath, filename, e);
                return HttpResponse::InternalServerError().json(format!("Failed to create file: {}", filename));
            }
        };

        // Write the file data to disk
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            if let Err(e) = f.write_all(&data) {
                error!("Failed to write to file {} at {:?}: {}", filename, filepath, e);
                return HttpResponse::InternalServerError().json(format!("Failed to write to file: {}", filename));
            }
        }

        file_count += 1;
        info!("Successfully uploaded file: {}", filename);
    }

    info!("Uploaded {} files successfully.", file_count);
    HttpResponse::Ok().json(format!("{} files uploaded successfully", file_count))
}

async fn index() -> impl Responder {
    let html_content = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Upload Directory</title>
    </head>
    <body>
        <h1>Upload Directory (Multiple Files)</h1>
        <form action="/upload" method="post" enctype="multipart/form-data">
            <input type="file" name="files" multiple webkitdirectory mozdirectory>
            <button type="submit">Upload</button>
        </form>
    </body>
    </html>
    "#;

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::init();

    info!("Starting the server...");

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))  // Serve the HTML page at root
            .route("/upload", web::post().to(save_file))  // Handle file upload
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
