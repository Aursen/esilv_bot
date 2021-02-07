mod client;

extern crate lazy_static;
use hyper::Client;
use hyper_tls::HttpsConnector;
use crate::client::DevinciClient;
use hyper::body::Bytes;
use lazy_static::lazy_static;
use leo_shared::MongoClient;
use std::collections::HashMap;
use tera::Context;
use tera::Tera;
use std::env;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        }
    };
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    if cfg!(debug_assertions) {
        dotenv::dotenv().expect("Failed to load .env file.");
    }

    let addr = "127.0.0.1:1337".parse().unwrap();
    let make_service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(response)) });

    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn response(req: Request<Body>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            if let Some(id) = get_queries(req.uri().query().unwrap_or("")).get("id") {
                let bdd = MongoClient::init().await.unwrap();
                let parsed_id = id.parse::<u64>().unwrap_or(0);
                let user = bdd.get_user(parsed_id).await.unwrap_or(None);
                if user.is_some() {
                    return default_message("Vous êtes déjà enregistré!");
                } else {
                    let mut resp = generate_tera_response("index.html", &Context::new())?;
                    resp.headers_mut().insert(
                        hyper::header::SET_COOKIE,
                        format!("user_id={}", parsed_id).parse().unwrap(),
                    );
                    return Ok(resp);
                }
            }
            return not_found();
        }
        (&Method::POST, "/login") => {
            let cookies = req
                .headers()
                .get_all(hyper::header::COOKIE)
                .iter()
                .find(|e| e.to_str().unwrap_or("").contains("user_id"));
            if let Some(c) = cookies {
                let id =
                    c.to_str().unwrap().split('=').collect::<Vec<_>>().to_vec()[1].parse::<u64>();
                if let Ok(parsed_id) = id {
                    let b = hyper::body::to_bytes(req).await?;
                    sign_in(parsed_id, b).await
                } else {
                    not_found()
                }
            } else {
                not_found()
            }
        }
        (&Method::GET, "/static/style.css") => Ok(stylesheet()),
        (&Method::GET, "/static/esilv.png") => Ok(logo()),
        _ => not_found(),
    }
}

fn stylesheet() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(Body::from(include_str!("assets/style.css")))
        .unwrap()
}

fn logo() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/png")
        .body(Body::from(include_bytes!("assets/esilv.png").to_vec()))
        .unwrap()
}

fn default_message(content: &str) -> Result<Response<Body>> {
    let mut context = Context::new();
    context.insert("message", content);
    generate_tera_response("default.html", &context)
}

fn generate_tera_response(path: &str, context: &Context) -> Result<Response<Body>> {
    Ok(Response::new(Body::from(
        TEMPLATES.render(path, context).unwrap(),
    )))
}

/// HTTP status code 404
fn not_found() -> Result<Response<Body>> {
    let mut context = Context::new();
    context.insert("message", "Erreur 404");
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(
            TEMPLATES.render("default.html", &context).unwrap(),
        ))
        .unwrap())
}

fn get_queries(query: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for queries in query.split('&').collect::<Vec<_>>().iter() {
        let mut splitted = queries.split('=');
        result.insert(
            splitted.next().unwrap().to_string(),
            (splitted.next().unwrap_or("")).to_string(),
        );
    }
    result
}

async fn sign_in(id: u64, form: Bytes) -> Result<Response<Body>> {
    let params = form_urlencoded::parse(&form)
        .into_owned()
        .collect::<HashMap<String, String>>();

    let username = match params.get("username") {
        Some(u) => u,
        _ => return not_found(),
    };

    let password = match params.get("password") {
        Some(p) => p,
        _ => return not_found(),
    };

    let bdd = MongoClient::init().await.unwrap();

    let mut client = DevinciClient::new();
    let devinci_user = match client.login(username, password).await {
        Ok(u) => Some(u),
        _ => None,
    };

    if let Some(mut u) = devinci_user {
        bdd.add_user(id, &mut u).await.unwrap();
        send_id(id).await.unwrap();
        default_message("Vous pouvez dorénavant retourner sur le Discord!")
    } else {
        let mut context = Context::new();
        context.insert("credentials_error", &true);
        generate_tera_response("index.html", &context)
    }
}

async fn send_id(id: u64) -> Result<()> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let webhook = env::var("WEBHOOK_URI").expect("You must set the WEBHOOK_URI environment var!");
    let req = Request::builder().method(Method::POST).uri(webhook).header(hyper::header::CONTENT_TYPE, "application/json").body(Body::from(format!("{{\"content\": \"{}\"}}", id))).unwrap();
    client.request(req).await.unwrap();
    Ok(())
}
