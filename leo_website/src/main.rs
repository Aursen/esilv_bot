mod adfs;
mod discord;

use crate::adfs::ADFSAuth;
use crate::discord::DiscordAuth;
use actix_files as fs;
use actix_session::{CookieSession, Session};
use actix_web::{
    client::Client,
    get,
    http::header::{CONTENT_TYPE, LOCATION},
    middleware::Logger,
    web, App, HttpResponse, HttpServer, Result,
};
use leo_shared::MongoClient;
use serde::Deserialize;
use serde_json::json;
use tera::{Context, Tera};

#[derive(Deserialize)]
struct Info {
    code: String,
}

const URL: &str = "https://discord-esilv.devinci.fr";

#[get("/adfs")]
async fn adfs_result(tmpl: web::Data<tera::Tera>, info: web::Query<Info>, session: Session) -> Result<HttpResponse> {
    let adfs_auth = ADFSAuth::new(URL);
    let id = session.get::<u64>("id").unwrap_or(Some(0_u64)).unwrap_or_default();

    if let Ok(token) = adfs_auth.get_token(&info.code).await {
        if let Ok(bdd) = MongoClient::init().await {
            if let Ok(mut u) = adfs_auth.get_devinci_user(&token).await {
                if bdd.add_user(id, &mut u).await.is_ok() {
                    let mut ctx = Context::new();
                    send_id(id).await?;
                    ctx.insert("message", "Vous pouvez retourner sur Discord!");
    
                    if let Ok(c) = tmpl.render("default.html", &ctx) {
                        return Ok(HttpResponse::Ok().content_type("text/html").body(c));
                    }
                }
            }
        }
    }
    Ok(HttpResponse::Found()
            .header(LOCATION, ADFSAuth::new(URL).generate_authorize_url())
            .finish())
}

#[get("/register")]
async fn register(
    tmpl: web::Data<tera::Tera>,
    info: web::Query<Info>,
    session: Session,
) -> Result<HttpResponse> {
    let discord_auth = DiscordAuth::new(&format!("{}/register", URL));

    if let Ok(token) = discord_auth.get_token(&info.code).await {
        if let Ok(id) = discord_auth.get_id(&token).await {
            if let Ok(bdd) = MongoClient::init().await {
                let parsed_id = id.parse::<u64>().unwrap_or(0);
                let user = bdd.get_user(parsed_id).await.unwrap_or(None);
                if user.is_some() {
                    session.set("id", &parsed_id)?;
                    let mut ctx = Context::new();
                    ctx.insert("message", "Vous êtes déjà enregistré! Vos rôles on été mis à jour!");
                    let content = tmpl.render("default.html", &ctx);
                    return match content {
                        Ok(c) => Ok(HttpResponse::Ok().content_type("text/html").body(c)),
                        Err(e) => Ok(HttpResponse::NotFound().body(e.to_string())),
                    };
                } else {
                    session.set("id", &parsed_id)?;
                    return Ok(HttpResponse::Found()
                        .header(LOCATION, ADFSAuth::new(URL).generate_authorize_url())
                        .finish());
                };
            }
        }
    }

    Ok(HttpResponse::Found().header(LOCATION, "/").finish())
}

#[get("/")]
async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .header(
            LOCATION,
            DiscordAuth::new(&format!("{}/register", URL)).generate_authorize_url(),
        )
        .finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if cfg!(debug_assertions) {
        dotenv::dotenv().expect("Failed to load .env file.");
        std::env::set_var("RUST_LOG", "actix_web=info");
        env_logger::init();
    }

    let addr = "127.0.0.1:8080";

    println!("Listening on http://{}", addr);

    HttpServer::new(|| {
        let tera = Tera::new("templates/**/*").unwrap();
        App::new()
            .data(tera)
            .wrap(Logger::default())
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .service(register)
            .service(adfs_result)
            .service(index)
            .service(fs::Files::new("/static", "static").show_files_listing())
            .default_service(web::route().to(|| HttpResponse::NotFound()))
    })
    .bind(addr)?
    .run()
    .await
}

async fn send_id(id: u64) -> Result<()> {
    let client = Client::new();
    let webhook =
        std::env::var("WEBHOOK_URI").expect("You must set the WEBHOOK_URI environment var!");

    let data = json!({ "content": id });

    client
        .post(webhook)
        .header(CONTENT_TYPE, "application/json")
        .send_json(&data)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::dev::Service;
    use actix_web::{http, test, App, Error};

    #[actix_rt::test]
    async fn test_index() -> Result<(), Error> {
        dotenv::dotenv().expect("Failed to load .env file.");

        let app = App::new().service(index);
        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::FOUND);

        Ok(())
    }

    #[actix_rt::test]
    async fn test_register() -> Result<(), Error> {
        let tera = Tera::new("templates/**/*").unwrap();
        dotenv::dotenv().expect("Failed to load .env file.");

        let app = App::new().data(tera).service(register);
        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/register").to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

        Ok(())
    }
}