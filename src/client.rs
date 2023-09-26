use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::Engine;
use eyre::{ContextCompat, Result, WrapErr};
use regex_lite::Regex;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate,
};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessExt, System, SystemExt};

use crate::types::{
    LolChatConversationMessageResource, LolChatConversationResource, LolLobbyLobbyDto,
};

fn find_process(system: &System) -> Result<String> {
    system
        .processes()
        .values()
        .find(|p| p.name() == "LeagueClientUx.exe")
        .map(|p| p.cmd().join(" "))
        .context("Failed to find LCU process")
}

type AuthToken = String;
type Port = u16;

fn extract_info(cmd_args: &str) -> Result<(AuthToken, Port)> {
    let port_re = Regex::new(r"--app-port=([0-9]*)").unwrap();
    let auth_token_re = Regex::new(r"--remoting-auth-token=([\w-]*)").unwrap();
    let port = port_re
        .captures(cmd_args)
        .and_then(|x| x.get(1))
        .map(|x| x.as_str().parse())
        .context("Failed to parse port")??;
    let auth_token = auth_token_re
        .captures(cmd_args)
        .and_then(|x| x.get(1))
        .map(|x| x.as_str().to_owned())
        .context("Failed to parse auth token")?;
    Ok((auth_token, port))
}

pub struct Client {
    port: Port,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let process = find_process(&sys)?;
        let (auth_token, port) = extract_info(&process)?;
        let encoded_auth_token = BASE64_STANDARD_NO_PAD.encode(format!("riot:{auth_token}"));

        let cert = Certificate::from_pem(include_bytes!("./riotgames.pem"))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(format!("Basic {encoded_auth_token}").as_str()).unwrap(),
        );
        let client = reqwest::blocking::ClientBuilder::new()
            .add_root_certificate(cert)
            .default_headers(headers)
            .build()?;

        Ok(Client { port, client })
    }

    fn get<T: for<'a> Deserialize<'a>>(&self, endpoint: &str) -> Result<T> {
        self.client
            .get(format!("https://127.0.0.1:{}{endpoint}", self.port))
            .send()?
            .json()
            .wrap_err("Failed to deserialize response")
    }

    fn post<T: for<'a> Deserialize<'a>, R: Serialize>(
        &self,
        endpoint: &str,
        body: &Option<R>,
    ) -> Result<T> {
        // let mut req_builder = self
        //     .client
        //     .post(format!("https://127.0.0.1:{}{endpoint}", self.port));
        //
        // if let Some(b) = body {
        //     req_builder = req_builder.json(&b);
        // }
        //
        // req_builder.send()?.json().into()
        self.client
            .post(format!("https://127.0.0.1:{}{endpoint}", self.port))
            .json(&body)
            .send()?
            .json()
            .wrap_err("Failed to deserialize response")
    }

    pub fn get_lol_lobby_v2_lobby(&self) -> Result<LolLobbyLobbyDto> {
        self.get("/lol-lobby/v2/lobby")
    }

    pub fn get_lol_chat_v1_conversations(&self) -> Result<Vec<LolChatConversationResource>> {
        self.get("/lol-chat/v1/conversations")
    }

    pub fn post_lol_chat_v1_conversations_by_id_messages(
        &self,
        id: &str,
        body: LolChatConversationMessageResource,
    ) -> Result<LolChatConversationMessageResource> {
        self.post(
            &format!("/lol-chat/v1/conversations/{id}/messages"),
            &Some(body),
        )
    }
}
