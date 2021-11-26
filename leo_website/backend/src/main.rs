#[macro_use]
extern crate rbatis;

mod models;
mod oauth;

use actix::Actor;
use actix_files::Files;
use actix_session::{CookieSession, Session};
use actix_web::{
    get,
    http::header::LOCATION,
    middleware::Logger,
    web::{self, Data},
    App, HttpResponse, HttpServer,
};
use rbatis::rbatis::Rbatis;
use serde::Deserialize;
use shared_lib::socket::{server::Server, session::tcp_server};
use std::{env, sync::Arc};

use crate::oauth::{adfs::ADFSAuth, discord::DiscordAuth};

#[derive(Deserialize)]
struct Info {
    code: String,
}

#[get("/adfs")]
async fn adfs_devinci(
    info: web::Query<Info>,
    session: Session,
    auth_devinci: Data<ADFSAuth>,
    rb: Data<Arc<Rbatis>>,
) -> actix_web::Result<HttpResponse> {
    let token = auth_devinci.get_token(&info.code).await?;
    session.insert("devinci_token", token)?;

    Ok(HttpResponse::Ok().body("TODO"))
}

#[get("/discord")]
async fn auth_discord(
    info: web::Query<Info>,
    session: Session,
    oauth_discord: Data<DiscordAuth>,
    auth_devinci: Data<ADFSAuth>,
) -> actix_web::Result<HttpResponse> {
    let token = oauth_discord.get_token(&info.code).await?;
    session.insert("discord_token", token)?;

    Ok(HttpResponse::Found()
        .append_header((LOCATION, auth_devinci.generate_authorize_url()))
        .finish())
}

#[get("/userinfo")]
async fn user_info(
    session: Session,
    oauth_discord: Data<DiscordAuth>,
    auth_devinci: Data<ADFSAuth>,
) -> actix_web::Result<HttpResponse> {
    if let Some(discord_token) = session.get::<String>("discord_token")? {
        if let Some(devinci_token) = session.get::<String>("devinci_token")? {
            let id = oauth_discord.get_id(&discord_token).await?;
            let mut user = auth_devinci.get_devinci_user(&devinci_token).await?;
            user.discord_id = id.parse::<u64>().unwrap_or_default();

            return Ok(HttpResponse::Ok().json(user));
        }
    }
    Ok(HttpResponse::Ok().finish())
}

#[get("/login")]
async fn login(auth: Data<DiscordAuth>) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .append_header((LOCATION, auth.generate_authorize_url()))
        .finish())
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv::dotenv().ok();
    env_logger::init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let db_url = env::var("DATABASE_URL").expect("database url");
    let host_url = env::var("HOST_URL").expect("You must set the HOST_URL environment var!");
    let redirect_discord = format!("{}/discord", host_url);

    let rb = Rbatis::new();
    rb.link(&db_url).await.expect("rbatis link database fail");

    // let user_test = DevinciUser {
    //     discord_id: 0,
    //     first_name: "jean".to_string(),
    //     last_name: "MARCHAND".to_string(),
    //     mail: "jean.marchand@edu.devinci.fr".to_string(),
    //     func: DevinciType::Student(2).into(),
    // };
    // rb.save(&user_test, &[]).await.unwrap();

    // let result: Option<DevinciUser> = rb.fetch_by_column("discord_id", &0).await.unwrap();

    // if let Some(user) = result {
    //     println!("{:#?}", DevinciType::from(user.func));
    // }

    let rb = Arc::new(rb);

    let server = Server::default().start();
    tcp_server("0.0.0.0:1234", server.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(server.to_owned()))
            .app_data(Data::new(rb.to_owned()))
            .app_data(Data::new(DiscordAuth::new(&redirect_discord)))
            .app_data(Data::new(ADFSAuth::new(&host_url)))
            .wrap(Logger::default())
            .wrap(CookieSession::private(&[0; 32]))
            .service(auth_discord)
            .service(login)
            .service(Files::new("/", env::var("FRONT_PATH").unwrap()).index_file("index.html"))
            .default_service(web::route().to(HttpResponse::NotFound))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
