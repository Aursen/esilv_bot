use awc::Client;
use form_urlencoded::byte_serialize;
use std::collections::HashMap;
use voca_rs::Voca;

use crate::models::{Claims, DevinciType, DevinciUser};

pub struct ADFSAuth {
    client_id: String,
    host_url: String,
    target_url: String,
}

impl ADFSAuth {
    pub fn new(url: &str) -> Self {
        let client_id = std::env::var("ADFS_DEVINCI_CLIENT_ID")
            .expect("You must set the ADFS_DEVINCI_CLIENT_ID environment var!");
        let target_url = std::env::var("ADFS_DEVINCI_URL")
            .expect("You must set the ADFS_DEVINCI_URL environment var!");

        Self {
            client_id,
            host_url: url.to_string(),
            target_url,
        }
    }

    pub fn generate_authorize_url(&self) -> String {
        let h_parsed_url: String = byte_serialize(self.host_url.as_bytes()).collect();
        let re_parsed_url: String =
            byte_serialize(format!("{}/adfs", self.host_url).as_bytes()).collect();
        format!(
            "{}/authorize?response_type=code&client_id={}&resource={}&redirect_uri={}",
            self.target_url, self.client_id, h_parsed_url, re_parsed_url
        )
    }

    pub async fn get_token(&self, code: &str) -> actix_web::Result<String> {
        let redirect = format!("{}/adfs", self.host_url);
        let mut data = HashMap::<&str, &str>::new();

        data.insert("grant_type", "authorization_code");
        data.insert("client_id", &self.client_id);
        data.insert("code", code);
        data.insert("redirect_uri", &redirect);

        let mut response = match Client::new()
            .post(format!("{}/token", self.target_url))
            .send_form(&data)
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(actix_web::error::ErrorBadRequest(e)),
        };

        let body = response.body().await?;
        let json: serde_json::Value = serde_json::from_slice(&body)?;

        match &json["access_token"] {
            serde_json::Value::String(t) => Ok(t.to_string()),
            e => Ok(e.to_string()),
        }
    }

    pub async fn get_devinci_user(&self, token: &str) -> actix_web::Result<DevinciUser> {
        let infos = token.split('.').collect::<Vec<_>>();

        if infos.len() < 2 {
            return Err(actix_web::error::ContentTypeError::ParseError.into());
        }

        let decoded = base64::decode(infos[1]).unwrap();
        let user: Claims = serde_json::from_slice(&decoded)?;

        let devinci_type = match user.group {
            Some(p) if p == "staff" || p == "intervenant" => DevinciType::Professor,
            Some(e) if e.starts_with("etu-esilv") => {
                let year = e.chars().last().unwrap_or('1').to_digit(10).unwrap_or(1);
                DevinciType::Student(year as u8)
            }
            _ => DevinciType::Other,
        };

        Ok(DevinciUser {
            discord_id: 0,
            first_name: user.given_name,
            last_name: user.family_name._capitalize(true),
            mail: user.email,
            func: devinci_type.into(),
        })
    }
}
