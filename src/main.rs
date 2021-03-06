extern crate dotenv;

use actix_files::{Files, NamedFile};
use actix_web::http::StatusCode;
use actix_web::{get, guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use dotenv::dotenv;
use mongodb::bson::{self, doc, Bson, Document};
use mongodb::options::{ClientOptions, ResolverConfig};
use std::path::{Path, PathBuf};

use handlebars::{Context, Handlebars, Helper, Output, RenderContext, RenderError};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::collections::HashMap;

// Uses mongodb 1.2.0 as actix-web uses a tokio version of 0.2.x, and mongodb crate
// upgrades to tokio 1.2.0 which is incompatible
#[get("/")]
async fn index(_req: HttpRequest) -> Result<NamedFile, Error> {
    println!("hello index");
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/index.html");
    Ok(NamedFile::open(path)?)
}

#[derive(Serialize, Deserialize, Debug)]
struct Timesheet {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,
    creation_date: bson::DateTime,
    random_path: String,
    name: String,
    email: String,
    namespace: String,
    client_name: String,
    client_contact_person: String,
    address: String,
    timesheet: String,
}

async fn find_record_from_random_string(
    path: &String,
) -> Result<Option<Document>, Box<dyn std::error::Error>> {
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

    let document: Option<Document> = collection.find_one(filter, None).await?;

    Ok(document)
}

#[derive(Deserialize)]
struct Info {
    identifier: String,
}

// catch only routes that don't end with .* so that the assets don't
// get resolved by this route and 404
#[get("/{identifier:\\w+$}")]
async fn timesheet(info: web::Path<Info>, hb: web::Data<Handlebars<'_>>) -> HttpResponse {
    println!("info: {:#?}", info.identifier);
    let document = find_record_from_random_string(&info.identifier)
        .await
        .expect("Failed to fetch record from mongodb");

    match document {
        None => {
            let data = json!({
                "error": "error",
                "status_code": 401
            });
            let body = hb.render("404", &data).unwrap();
            HttpResponse::Ok().body(body)
        }
        Some(doc) => {
            let sheet: Timesheet = bson::from_bson(Bson::Document(doc))
                .expect("Couldn't parse Timesheet struct from document");
            let timesheet_json: Vec<HashMap<String, i32>> = serde_json::from_str(&sheet.timesheet)
                .expect("Couldn't parse timesheet json from document");

            let data = json!({
                "id": sheet.id,
                "creation_date": sheet.creation_date.date().format("%B").to_string(),
                "random_path": sheet.random_path,
                "name": sheet.name,
                "email": sheet.email,
                "namespace": sheet.namespace,
                "client_name": sheet.client_name,
                "client_contact_person": sheet.client_contact_person,
                "address": sheet.address,
                "timesheet": timesheet_json,
            });

            let body = hb.render("timesheet", &data).unwrap();
            HttpResponse::Ok().body(body)
        }
    }
}

async fn p404() -> Result<NamedFile, Error> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/404.html");
    Ok(NamedFile::open(path)?.set_status_code(StatusCode::NOT_FOUND))
}

fn increment_index(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let v = h
        .param(0)
        .map(|v| v.value())
        .ok_or(RenderError::new("param not found"));
    let value = v.unwrap().as_u64().unwrap() + 1 as u64;
    out.write(&*value.to_string())?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let mut handlebars = Handlebars::new();
    handlebars.register_helper("increment_index", Box::new(increment_index));
    handlebars
        .register_templates_directory(".html", "./dist")
        .unwrap();
    let handlebars_ref = web::Data::new(handlebars);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(handlebars_ref.clone())
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
