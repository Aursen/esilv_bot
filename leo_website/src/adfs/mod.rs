use actix_web::client::Client;
use form_urlencoded::byte_serialize;
use leo_shared::user::{DevinciType, DevinciUser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use voca_rs::*;

pub struct ADFSAuth {
    client_id: String,
    host: String,
    redirect: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    aud: String,
    iss: String,
    iat: usize,
    exp: usize,
    email: String,
    family_name: String,
    given_name: String,
    sub: String,
    group: Option<String>,
    auth_time: String,
    authmethod: String,
    ver: String,
    appid: String,
}

impl ADFSAuth {
    pub fn new(host: &str) -> Self {
        let client_id = std::env::var("ADFS_CLIENT_ID")
            .expect("You must set the ADFS_CLIENT_ID environment var!");

        Self {
            client_id,
            host: String::from(host),
            redirect: format!("{}/adfs", host),
        }
    }

    pub fn generate_authorize_url(&self) -> String {
        let h_parsed_url: String = byte_serialize(self.host.as_bytes()).collect();
        let re_parsed_url: String = byte_serialize(self.redirect.as_bytes()).collect();
        format!("https://adfs.devinci.fr/adfs/oauth2/authorize?response_type=code&client_id={}&resource={}&redirect_uri={}", self.client_id, h_parsed_url, re_parsed_url)
    }

    pub async fn get_token(&self, code: &str) -> actix_web::Result<String> {
        let mut data = HashMap::<&str, &str>::new();
        data.insert("grant_type", "authorization_code");
        data.insert("client_id", &self.client_id);
        data.insert("code", code);
        data.insert("redirect_uri", &self.redirect);

        let mut response = Client::new()
            .post("https://adfs.devinci.fr/adfs/oauth2/token")
            .send_form(&data)
            .await?;
        let body = response.body().await?;
        let json: serde_json::Value = serde_json::from_slice(&body)?;

        match &json["access_token"] {
            serde_json::Value::String(t) => Ok(t.to_string()),
            e => Ok(e.to_string()),
        }
    }

    pub async fn get_devinci_user(&self, token: &str) -> actix_web::Result<DevinciUser> {
        let infos = token.split('.').collect::<Vec<_>>()[1];
        let decoded = base64::decode(infos).unwrap_or_default();
        let user: Claims = serde_json::from_slice(&decoded)?;

        let devinci_type = match user.group {
            Some(g) => match g.as_str() {
                "staff" | "intervenant" => DevinciType::Professor,
                e if e.starts_with("etu-esilv") => {
                    let year = e.chars().last().unwrap_or('1').to_digit(10).unwrap_or(1);
                    DevinciType::Student(year as i32)
                }
                _ => DevinciType::Other,
            },
            _ => DevinciType::Other,
        };

        Ok(DevinciUser::new(
            user.given_name,
            user.family_name._capitalize(true),
            user.email,
            devinci_type,
        ))
    }
}
