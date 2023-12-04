use std::collections::HashMap;

use serde::Deserialize;

pub struct Bridge {
    address: String,
    username: String,
    client: reqwest::Client,
}

#[derive(Debug)]
pub enum RegisterError {
    UrlParsing,
    SendingRequest(reqwest::Error),
    Other,
    HueError { description: String, id: usize },
}

#[derive(Debug, Deserialize)]
struct RegisterResponseError {
    pub address: String,
    pub description: String,
    pub r#type: usize,
}
#[derive(Debug, Deserialize)]
struct RegisterResponseSuccess {
    username: String,
}

#[derive(Debug, Deserialize)]
pub struct Light {
    pub capabilities: LightCapabilities,
    pub config: LightConfig,
    pub manufacturername: String,
    pub modelid: String,
    pub name: String,
    pub productid: String,
    pub productname: String,
    pub state: LightState,
    pub swconfigid: String,
    pub swupdate: serde_json::Value,
    pub swversion: String,
    pub r#type: String,
    pub uniqueid: String,
}

#[derive(Debug, Deserialize)]
pub struct LightCapabilities {
    pub certified: bool,
    pub control: serde_json::Value,
    pub streaming: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct LightConfig {
    pub archetype: String,
    pub direction: String,
    pub function: String,
    pub startup: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct LightState {
    pub alert: String,
    pub bri: usize,
    pub colormode: String,
    pub ct: usize,
    pub effect: String,
    pub hue: usize,
    pub mode: String,
    pub on: bool,
    pub reachable: bool,
    pub sat: usize,
    pub xy: serde_json::Value,
}

impl Bridge {
    pub async fn register<A>(client: &reqwest::Client, address: A) -> Result<String, RegisterError>
    where
        A: AsRef<str>,
    {
        let url = reqwest::Url::parse(&format!("http://{}/api", address.as_ref()))
            .map_err(|e| RegisterError::UrlParsing)?;
        let body = serde_json::json!({
            "devicetype": "phue-exporter",
        });

        let res = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(RegisterError::SendingRequest)?;

        let status = res.status();

        if !status.is_success() {
            return Err(RegisterError::Other);
        }

        let content: Vec<HashMap<String, serde_json::Value>> =
            res.json().await.map_err(|e| RegisterError::Other)?;

        let entry = content.first().ok_or_else(|| RegisterError::Other)?;

        // [src/lib.rs:40] &content = Array [
        // Object {
        //     "success": Object {
        //         "username": String("XBbpm9HlERlh0tf0dtSDvgTNaJAlLznhlZcgmWsQ"),
        //     },
        // },
        // ]

        if let Some(obj) = entry.get("success") {
            let parsed = match serde_json::from_value::<RegisterResponseSuccess>(obj.clone()) {
                Ok(p) => p,
                Err(e) => return Err(RegisterError::Other),
            };

            return Ok(parsed.username);
        }

        if let Some(obj) = entry.get("error") {
            let parsed = match serde_json::from_value::<RegisterResponseError>(obj.clone()) {
                Ok(p) => p,
                Err(e) => return Err(RegisterError::Other),
            };

            return Err(RegisterError::HueError {
                description: parsed.description,
                id: parsed.r#type,
            });
        }

        Err(RegisterError::Other)
    }

    pub fn new<A, U>(client: reqwest::Client, addr: A, username: U) -> Self
    where
        A: Into<String>,
        U: Into<String>,
    {
        Self {
            address: addr.into(),
            username: username.into(),
            client,
        }
    }

    pub async fn read_config(&self) -> Result<serde_json::Value, ()> {
        let uri = reqwest::Url::parse(&format!(
            "http://{}/api/{}/config",
            self.address, self.username
        ))
        .map_err(|_| ())?;

        let resp = self.client.get(uri).send().await.map_err(|_| ())?;

        let status = resp.status();
        if !status.is_success() {
            return Err(());
        }

        resp.json().await.map_err(|_| ())
    }

    pub async fn lights(&self) -> Result<HashMap<String, Light>, ()> {
        let uri = reqwest::Url::parse(&format!(
            "http://{}/api/{}/lights",
            self.address, self.username
        ))
        .map_err(|_| ())?;

        let resp = self.client.get(uri).send().await.map_err(|_| ())?;

        let status = resp.status();
        if !status.is_success() {
            return Err(());
        }

        resp.json().await.map_err(|_| ())
    }
}
