use actix_files::{Files, NamedFile};
use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use std::path::PathBuf;

async fn index(req: HttpRequest) -> Result<NamedFile, Error> {
    let path: PathBuf = req.match_info().query("index.html").parse().unwrap();
    println!("{:#?}", req.resource_map());
    let file = NamedFile::open(path)?;
    Ok(file)
}

// async fn timesheet(req: HttpRequest) -> Result<NamedFile, Box<dyn Error>> {
//     let path: PathBuf = req.match_info().query("filename").parse().unwrap();
//     Ok(NamedFile::open(path)?)
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(Files::new(
                "/dist",
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dist"),
            ))
            .service(web::resource("/").route(web::get().to(index)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
