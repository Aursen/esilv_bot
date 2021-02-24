use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::Client;
use hyper::{Body, Method, Request};
use hyper_tls::HttpsConnector;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;
use leo_shared::user::{DevinciUser, DevinciType};

//Maybe uses https://github.com/atroche/rust-headless-chrome

#[derive(Debug)]
pub struct DevinciClient {
    client: Client<HttpsConnector<HttpConnector>>,
    cookies: HashMap<String, String>,
    referer: String,
}

impl DevinciClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build::<_, Body>(HttpsConnector::new()),
            cookies: HashMap::new(),
            referer: String::new(),
        }
    }

    fn generate_cookies(&mut self) -> String {
        self.cookies
            .iter()
            .map(|i| format!("{}={}", i.0, i.1))
            .collect::<Vec<_>>()
            .join("; ")
    }

    async fn send(
        &mut self,
        method: Method,
        uri: &str,
        content: Option<String>,
    ) -> Result<(Option<String>, String), Box<dyn Error>> {
        let mut req_header = Request::builder().method(&method).uri(uri);
        if method == Method::POST {
            req_header = req_header.header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            );
        }

        if !self.referer.is_empty() {
            req_header = req_header.header("Referer", &self.referer);
        }

        if !self.cookies.is_empty() {
            req_header = req_header.header("Cookie", &self.generate_cookies());
        }

        let req = match content {
            Some(b) => req_header.body(Body::from(b))?,
            None => req_header.body(Body::empty())?,
        };

        let resp = self.client.request(req).await?;

        let redir = if let Some(c) = resp.headers().get("location") {
            Some(c.to_str()?.to_string())
        } else {
            None
        };

        self.referer = uri.to_string();

        let cookies = resp.headers().get_all("set-cookie");
        let re = Regex::new(r"(\S*)=(\S*);")?;
        for cookie in cookies {
            let cap = re.captures(cookie.to_str()?).unwrap();
            self.cookies.insert(cap[1].to_string(), cap[2].to_string());
        }

        let mut body = String::new();
        hyper::body::aggregate(resp.into_body())
            .await?
            .reader()
            .read_to_string(&mut body)?;

        Ok((redir, body))
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<DevinciUser, Box<dyn Error>> {
        let (redir, _) = self
            .send(
                Method::GET,
                &format!(
                    "https://www.leonard-de-vinci.net/login.sso.php?username={}",
                    username
                ),
                None,
            )
            .await?;
        let url = redir.unwrap();
        self.send(Method::GET, &url, None).await?;
        let content = format!(
            "UserName={}&Password={}&AuthMethod=FormsAuthentication",
            username, password
        );
        self.send(Method::POST, &url, Some(content)).await?;
        let (_, body) = self.send(Method::GET, &url, None).await?;
        let mut re = Regex::new(r#"value="(\S+)""#)?;
        let cap = re.captures(&body).unwrap();
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("SAMLResponse", &cap[1])
            .append_pair(
                "RelayState",
                "https://www.leonard-de-vinci.net/login.sso.php",
            )
            .finish();

        let (redir, _) = self.send(Method::POST, "https://www.leonard-de-vinci.net/include/SAML/module.php/saml/sp/saml2-acs.php/devinci-sp", Some(encoded)).await?;

        if let Some(link) = &redir {
            self.send(Method::GET, link, None).await?;
        }else{
            return Err("Credentials error".into())
        }

        let (_, body) = self
            .send(Method::GET, "https://www.leonard-de-vinci.net", None)
            .await?;

        re = Regex::new(r"<header>([A-Z]+) ANNEE (.)")?;

        let func = if re.is_match(&body) {
            if let Some(cap) = re.captures(&body){
                if &cap[1] == "ESILV"{
                    DevinciType::Student(cap[2].parse::<i32>()?)
                }else{
                    DevinciType::Other
                }
            }else{
                return Err("Regex error".into())
            }

        } else {
            DevinciType::Professor
        };
        
        //re = Regex::new(r#"<span style="opacity: 1;">([\w\s-]+)\s([A-Z\s]+)</span>"#)?;
        //re = Regex::new(r#"<div>(Monsieur|Madame)\s([\w\s-]+)\s([A-Z\s]+)\s</div>"#)?;

        re = Regex::new(r"([\w])(\w+)")?;

        let mut cred = username.split('@').next().unwrap().split('.');

        let cap1 = re.captures(&cred.next().unwrap()).unwrap();
        let cap2 = re.captures(&cred.next().unwrap()).unwrap();

        let first_name = format!("{}{}", &cap1[1].to_uppercase(), &cap1[2]);
        let last_name = format!("{}{}", &cap2[1].to_uppercase(), &cap2[2]);
        Ok(DevinciUser::new(first_name, last_name, (&username).to_string(), func))
    }
}
