 
use select::predicate::{And, Class, Name};
use select::document::Document;

use reqwest;

use w3g_common::errors::Result;
 
use w3g_common::pubsub::model::Player; 

use std::collections::HashMap;

#[derive(Debug)]
struct GameLobby<'a> {
    game_id: &'a str,
    bot_id: &'a str,
    num_players: u8,
    total_players: u8,
    is_lobby: bool,
    game_name: &'a str,
}

impl<'a> GameLobby<'a>
{
    fn from_str(raw: &'a str) -> Result<GameLobby<'a>>
    {
        let mut splits = raw.splitn(6, '|');

        let game_id = splits.next().ok_or("No game_id")?;
        let bot_id = splits.next().ok_or("No bot_id")?;
        let num_players = splits.next().ok_or("No num_players")?.parse::<u8>()?;
        let total_players = splits.next().ok_or("No total_players")?.parse::<u8>()?;
        let is_lobby = splits.next().ok_or("No is_lobby")?.parse::<u8>()? == 1;
        let game_name = splits.next().ok_or("No game_name")?;

        Ok(
            GameLobby {
                game_id,
                bot_id,
                num_players,
                total_players,
                is_lobby,
                game_name,
            }
        )
    }
}

fn get_lobby_for_bot<'a>(bot_id: &str, raw: &'a str) -> Result<GameLobby<'a>>
{
    for line in raw.lines()
    {
        if let Ok(lobby) = GameLobby::from_str(line)
        {
            if lobby.bot_id == bot_id
            {
                return Ok(lobby);
            }
        }
    }

    bail!("Bot id: {} was not found", bot_id);
}

fn get_players_for_lobby(raw: &str) -> Result<HashMap<u32, Player>>
{
    let mut players = HashMap::new();
    let mut index = 0;

    let td_and_slot = And(Name("td"), Class("slot"));

    let lobby_dom = Document::from(raw);
    for tr in lobby_dom.find(Name("tr"))
    {
        let mut slots = tr.find(td_and_slot);
 
        let name = match slots.next()
        {
            None => continue,
            Some(name) =>
            {
                if name.text() == "Empty"
                { 
                    index = index + 1;
                    continue;
                }

                match name.find(Name("a")).next()
                {
                    None => continue,
                    Some(name) => name.text(),
                }
            }
        };

        let realm = match slots.next().map(|realm| realm.text())
        {
            None => continue,
            Some(realm) =>
            {
                match realm.as_str()
                {
                    /* db.playerStats.distinct("player.realm") */
                    "USEast" => "useast.battle.net".to_string(),
                    "USWest" => "uswest.battle.net".to_string(),
                    "Europe" => "europe.battle.net".to_string(),
                    "Asia" => "asia.battle.net".to_string(),
                    "entconnect" => "entconnect".to_string(),
                    _ =>
                    {
                        warn!("Found unknown realm: {} will likely cause errors for stats lookup", realm);
                        realm
                    },
                }
            },
        };

        // let ping = slots.next();
        // let elo = slots.next();
        // let win_loss = slots.next();

        players.insert(index, Player::new(name, realm));
        index = index + 1;
    }
 
    Ok(players)
}

pub fn get_players_for_bot(bot_id: &str) -> Result<HashMap<u32, Player>>
{
    // Get current game_id
    let game_ids_url = "https://entgaming.net/forum/games_fast.php";
    let mut games_response = reqwest::get(game_ids_url)?;
    if !games_response.status().is_success()
    {
        bail!("Bad games_list status: {}", games_response.status());
    }

    let games_text = games_response.text()?;

    let id_lobby = get_lobby_for_bot(bot_id, &games_text)?;

    // Get players in that game
    let game_by_id_url = format!("https://entgaming.net/forum/slots_fast.php?id={}", id_lobby.game_id);
    let mut lobby_response = reqwest::get(&game_by_id_url)?;
    if !lobby_response.status().is_success()
    {
        bail!("Bad lobby status: {}", lobby_response.status());
    }

    let lobby_text = lobby_response.text()?;

    get_players_for_lobby(&lobby_text)
}