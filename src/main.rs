use actix_web::{web, App, HttpServer, Responder};

async fn index(info: web::Path<(String, u32)>) -> impl Responder {
    format!("Hello {}! id:{}", info.0, info.1)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(web::resource("/{name}/{id}/index.html").to(index))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
