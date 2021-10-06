extern crate dotenv;

use actix_files::{Files, NamedFile};
use actix_web::{get, middleware, web, App, Error, HttpRequest, HttpServer};
use dotenv::dotenv;
use futures::TryStreamExt;
use mongodb::options::{ClientOptions, ResolverConfig};
use mongodb::{bson::doc, options::FindOptions};
use serde::Deserialize;
use std::path::{Path, PathBuf};

// Uses mongodb 1.2.0 as actix-web uses a tokio version of 0.2.x, and mongodb crate
// upgrades to tokio 1.2.0 which is incompatible
#[get("/")]
async fn index(_req: HttpRequest) -> Result<NamedFile, Error> {
    println!("hello index");
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/index.html");
    Ok(NamedFile::open(path)?)
}

async fn find_record_from_random_string(path: &String) -> Result<(), Box<dyn std::error::Error>> {
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
    println!("{:#?}", filter);
    let document = collection
        .find_one(filter, None)
        .await?
        .expect("Missing document");

    println!("{:#?}", document);

    Ok(())
}

#[derive(Deserialize)]
struct Info {
    timesheet: String,
}

#[get("/{timesheet}")]
async fn timesheet(info: web::Path<Info>) -> Result<NamedFile, Error> {
    println!("hello timesheet");
    find_record_from_random_string(&info.timesheet)
        .await
        .unwrap_or_else(|err| {
            eprintln!("Error fetching from mongodb: {}", err);
        });
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/timesheet.html");
    Ok(NamedFile::open(path)?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            // .wrap(middleware::Logger::default())
            .service(index)
            .service(timesheet)
            .service(Files::new("/", "./dist/"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
