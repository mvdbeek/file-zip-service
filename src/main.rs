use actix_web::{get, web, App, HttpServer, Responder, HttpResponse, HttpRequest};
use clap::{App as ClapApp, Arg};
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


struct Config {
    host: String,
    port: u16,
    workers: usize,
}

fn parse_command_line_args() -> Config {
    let matches = ClapApp::new("File Zip Service")
        .version("1.0")
        .author("Your Name")
        .about("Creates a zip archive from specified files.")
        .arg(
            Arg::with_name("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .help("Sets the server host")
                .takes_value(true)
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the server port")
                .takes_value(true)
                .default_value("8080"),
        )
        .arg(
            Arg::with_name("workers")
                .short('w')
                .long("workers")
                .value_name("NUM")
                .help("Sets the number of worker threads")
                .takes_value(true)
                .default_value("4"),
        )
        .get_matches();

    // Extract the host, port, and workers value from command line arguments
    let host = matches.value_of("host").unwrap_or("0.0.0.0").to_string();
    let port: u16 = matches
        .value_of("port")
        .unwrap_or("8080")
        .parse()
        .expect("Invalid port number");
    let workers = matches
        .value_of("workers")
        .unwrap_or("4")
        .parse()
        .expect("Invalid number of workers");

    Config { host, port, workers }
}




#[get("/download")]
async fn download_files(_req: HttpRequest, req_body: web::Json<Vec<FileRequest>>) -> impl Responder {
    download_files_handler(req_body).await
}



async fn download_files_handler(req_body: web::Json<Vec<FileRequest>>) -> HttpResponse {
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
    let config = parse_command_line_args();

    HttpServer::new(|| {
        App::new().service(download_files)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .workers(config.workers)
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use serde_json::json;
    use std::io::Write;
    use tempfile::tempdir;


    #[actix_rt::test]
    async fn test_create_zip_archive() {
        // Arrange: Create temporary files and directories
        let dir = tempdir().expect("Failed to create temporary directory");
        let file1_path = dir.path().join("file1.txt");
        let file2_path = dir.path().join("file2.txt");

        let mut file1 = std::fs::File::create(&file1_path).expect("Failed to create file1.txt");
        file1.write_all(b"Content of file1.txt").expect("Failed to write to file1.txt");

        let mut file2 = std::fs::File::create(&file2_path).expect("Failed to create file2.txt");
        file2.write_all(b"Content of file2.txt").expect("Failed to write to file2.txt");

        // JSON request body with temporary file paths
        let json_body = json!([
            {"path": file1_path.to_str().unwrap(), "arcname": "dir/file1.txt"},
            {"path": file2_path.to_str().unwrap(), "arcname": "dir/file2.txt"}
        ]);

        // Act: Send POST request to /download endpoint
        let app = test::init_service(App::new().service(web::resource("/download").route(web::post().to(download_files_handler)))).await;
        let request = test::TestRequest::post().uri("/download").set_json(&json_body).to_request();
        let response: HttpResponse = test::call_service(&app, request).await.into();

        // Assert: Check the response status code
        assert_eq!(response.status(), StatusCode::OK);

        // Clean up: Temporary files and directories will be deleted automatically when they go out of scope
    }
}
