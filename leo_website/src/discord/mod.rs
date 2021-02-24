use std::error::Error;
use std::collections::HashMap;
use actix_web::http::header::{CONTENT_TYPE, AUTHORIZATION};
use actix_web::client::Client;
use form_urlencoded::byte_serialize;

pub struct DiscordAuth {
    client_id: String,
    client_secret: String,
    redirect: String
}

impl DiscordAuth {
    pub fn new(redirect: &str) -> Self {
        let client_id = std::env::var("DISCORD_CLIENT_ID").expect("You must set the DISCORD_CLIENT_ID environment var!");
        let client_secret = std::env::var("DISCORD_CLIENT_SECRET").expect("You must set the DISCORD_CLIENT_SECRET environment var!");

        Self {
            client_id,
            client_secret,
            redirect: String::from(redirect)
        }
    }

    pub fn generate_authorize_url(&self) -> String {
        let parsed_url: String = byte_serialize(self.redirect.as_bytes()).collect();
        format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify", self.client_id, parsed_url)
    }

    pub async fn get_token(&self, code: &str) -> Result<String, Box<dyn Error>> {
        let mut data = HashMap::<&str, &str>::new();
        data.insert("client_id", &self.client_id);
        data.insert("client_secret", &self.client_secret);
        data.insert("grant_type", "authorization_code");
        data.insert("code", code);
        data.insert("redirect_uri", &self.redirect);
        data.insert("scope", "identify");

        let client = Client::new();

        let mut response = client.post("https://discord.com/api/oauth2/token").header(CONTENT_TYPE, "application/x-www-form-urlencoded").send_form(&data).await.unwrap();
        
        let body = response.body().await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        match &json["access_token"] {
            serde_json::Value::String(t) => Ok(t.to_string()),
            _ => Err("Invalid value")?
        }
    }

    pub async fn get_id(&self, token: &str) -> Result<String, Box<dyn Error>> {
        let client = Client::new();
        let mut response = client.get("https://discord.com/api/users/@me").header(AUTHORIZATION, format!("Bearer {}", token)).send().await.unwrap();

        let body = response.body().await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        match &json["id"] {
            serde_json::Value::String(t) => Ok(t.to_string()),
            _ => Err("Invalid value")?
        }
    }
}