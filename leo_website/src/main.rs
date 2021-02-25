mod discord;

use crate::discord::DiscordAuth;
use actix_files as fs;
use actix_session::{CookieSession, Session};
use actix_web::{
    client::Client,
    get,
    http::header::{CONTENT_TYPE, LOCATION},
    middleware::Logger,
    post, web, App, HttpResponse, HttpServer, Result,
};
use leo_auth::DevinciClient;
use leo_shared::MongoClient;
use serde::Deserialize;
use serde_json::json;
use tera::{Context, Tera};
use tokio::runtime::Runtime;

#[derive(Deserialize)]
struct Info {
    code: String,
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[get("/register")]
async fn register(
    tmpl: web::Data<tera::Tera>,
    info: web::Query<Info>,
    session: Session,
) -> Result<HttpResponse> {
    let discord_auth = DiscordAuth::new("https://discord-esilv.devinci.fr/register");

    if let Ok(token) = discord_auth.get_token(&info.code).await {
        if let Ok(id) = discord_auth.get_id(&token).await {
            if let Ok(bdd) = MongoClient::init().await {
                let parsed_id = id.parse::<u64>().unwrap_or(0);
                let user = bdd.get_user(parsed_id).await.unwrap_or(None);
                let content = if user.is_some() {
                    let mut ctx = Context::new();
                    ctx.insert("message", "Vous êtes déjà enregistré!");
                    tmpl.render("default.html", &ctx)
                } else {
                    session.set("id", &parsed_id)?;
                    tmpl.render("index.html", &Context::new())
                };
                return match content {
                    Ok(c) => Ok(HttpResponse::Ok().content_type("text/html").body(c)),
                    Err(e) => Ok(HttpResponse::NotFound().body(e.to_string())),
                }
            }
        }
    }

    Ok(HttpResponse::Found().header(LOCATION, "/").finish())
}

#[post("/login")]
async fn login(
    tmpl: web::Data<tera::Tera>,
    info: web::Form<Credentials>,
    session: Session,
) -> Result<HttpResponse> {
    let id = session.get::<u64>("id").unwrap_or(Some(0_u64)).unwrap();

    let bdd = MongoClient::init().await.unwrap();

    let mut client = DevinciClient::new();

    let rt = Runtime::new().unwrap();
    let devinci_user = rt.block_on(async { client.login(&info.username, &info.password).await });

    let content = if let Ok(mut u) = devinci_user {
        let mut ctx = Context::new();
        if bdd.add_user(id, &mut u).await.is_ok() {
            send_id(id).await.unwrap();
            ctx.insert("message", "Vous êtes déjà enregistré!");
            tmpl.render("default.html", &ctx)
        } else {
            ctx.insert("credentials_error", &true);
            tmpl.render("index.html", &ctx)
        }
    } else {
        let mut ctx = Context::new();
        ctx.insert("credentials_error", &true);
        tmpl.render("index.html", &ctx)
    };

    match content {
        Ok(c) => Ok(HttpResponse::Ok().content_type("text/html").body(c)),
        Err(e) => Ok(HttpResponse::NotFound().body(e.to_string())),
    }
}

#[get("/")]
async fn index() -> Result<HttpResponse> {
    let discord_auth = DiscordAuth::new("https://discord-esilv.devinci.fr/register");
    Ok(HttpResponse::Found()
        .header(LOCATION, discord_auth.generate_authorize_url())
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
            .service(login)
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
