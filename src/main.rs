use eyre::{ContextCompat, Result};
use itertools::Itertools;
use rand::prelude::*;
use std::thread::sleep;
use std::time::Duration;

use crate::client::Client;
use crate::types::LolChatConversationMessageResource;

mod client;
mod types;

fn main() -> Result<()> {
    let client = Client::new()?;

    // Create teams
    let lobby = client.get_lol_lobby_v2_lobby()?;
    let mut players: Vec<&str> = lobby
        .members
        .iter()
        .map(|p| p.summoner_name.as_ref())
        .collect();
    players.shuffle(&mut thread_rng());
    let (team1, team2) = players
        .chunks((players.len() / 2) + (players.len() % 2))
        .map(|x| x.join("\n"))
        .collect_tuple()
        .context("Failed to create two teams")?;
    let teams_output = format!(".\nTeam 1:\n{team1}\n----------\nTeam 2:\n{team2}");
    sleep(Duration::new(10, 0));

    // Find custom game chat
    let conversations = client.get_lol_chat_v1_conversations()?;

    let custom_game_chat = conversations
        .iter()
        .find(|x| x.type_ == "customGame")
        .context("Failed to find custom game chat")?;

    // Post teams in chat
    let post_body = LolChatConversationMessageResource {
        body: teams_output,
        type_: "groupchat".to_string(),
        ..Default::default()
    };
    client.post_lol_chat_v1_conversations_by_id_messages(&custom_game_chat.id, post_body)?;

    Ok(())
}
