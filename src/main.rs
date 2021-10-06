extern crate dotenv;

use actix_files::{Files, NamedFile};
use actix_web::guard::Guard;
use actix_web::http::StatusCode;
use actix_web::{
    error, get, guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer,
};
use dotenv::dotenv;
use futures::TryStreamExt;
use mongodb::options::{ClientOptions, ResolverConfig};
use mongodb::{bson::doc, options::FindOptions};
use serde::Deserialize;
use std::io;
use std::path::{Path, PathBuf};

// Uses mongodb 1.2.0 as actix-web uses a tokio version of 0.2.x, and mongodb crate
// upgrades to tokio 1.2.0 which is incompatible
#[get("/")]
async fn index(_req: HttpRequest) -> Result<NamedFile, Error> {
    println!("hello index");
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/index.html");
    Ok(NamedFile::open(path)?)
}

async fn find_record_from_random_string(
    path: &String,
) -> Result<NamedFile, Box<dyn std::error::Error>> {
    dotenv().ok();

    // get the client uri
    let client_uri =
        std::env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var");

    //parse the connection string
    let options: ClientOptions =
        ClientOptions::parse_with_resolver_config(&client_uri, ResolverConfig::cloudflare())
            .await?;
    //get the client with the connection string
    let client = mongodb::Client::with_options(options)?;
    //find the collection
    let collection = client
        .database("timesheet-gen")
        .collection("timesheet-temp-paths");

    let filter = doc! { "random_path": &path };

    let document = collection.find_one(filter, None).await?;

    match document {
        None => {
            let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/404.html");
            Ok(NamedFile::open(path)?.set_status_code(StatusCode::NOT_FOUND))
        }
        Some(_) => {
            let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/timesheet.html");
            Ok(NamedFile::open(path)?)
        }
    }
}

#[derive(Deserialize)]
struct Info {
    identifier: String,
}

#[get("/{identifier:\\w+$}")]
async fn timesheet(info: web::Path<Info>) -> Result<NamedFile, Error> {
    println!("info: {:#?}", info.identifier);
    let named_file = find_record_from_random_string(&info.identifier)
        .await
        .expect("Failed to fetch record from mongodb");
    Ok(named_file)
}

async fn p404() -> Result<NamedFile, Error> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/404.html");
    Ok(NamedFile::open(path)?.set_status_code(StatusCode::NOT_FOUND))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(index)
            .service(timesheet)
            .service(Files::new("/", "./dist/"))
            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
