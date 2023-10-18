use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use zip::{write::FileOptions, ZipWriter};


#[derive(Debug, Serialize, Deserialize)]
struct FileRequest {
    path: String,
    arcname: String,
}


#[get("/download")]
async fn download_files(req_body: web::Json<Vec<FileRequest>>) -> impl Responder {
    // List of file paths to include in the zip archive

    // Create a buffer to store the zip archive
    let mut zip_buffer = Vec::new();
    {
        // Create a zip archive
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755); // Set desired permissions for the files in the archive

        // Add files to the zip archive
        for file_request in req_body.iter() {
            let path = &file_request.path;
            let arcname = &file_request.arcname;

            let mut file = File::open(path).expect("Failed to open file");
            let file_content = {
                let mut content = Vec::new();
                file.read_to_end(&mut content).expect("Failed to read file content");
                content
            };
            zip.start_file(arcname, options.clone()).expect("Failed to add file to zip");
            zip.write_all(&file_content).expect("Failed to write file content to zip");
        }
    }

    // Set appropriate response headers
    let response = web::Bytes::from(zip_buffer);
    HttpResponse::Ok()
        .content_type("application/zip")
        .body(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(download_files)
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
