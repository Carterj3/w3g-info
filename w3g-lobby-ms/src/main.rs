#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

extern crate chrono;

use chrono::Utc;

extern crate reqwest; 
extern crate select;

use select::predicate::{And, Class, Name};
use select::document::Document;

extern crate w3g_common;

use w3g_common::errors::Result;

use w3g_common::pubsub::PubSubConsumer;

use w3g_common::pubsub::PubSubProducer;

use w3g_common::pubsub::model::Player;

use w3g_common::pubsub::ID_BULK_STATS_REQUESTS_TOPIC;
use w3g_common::pubsub::ID_LOBBY_REQUESTS_TOPIC;
 
use std::env;
use std::collections::HashMap;
use std::thread;


const KAFKA_GROUP: &'static str = "w3g-lobby-ms";

fn lobby_request_handler(mut consumer: PubSubConsumer, mut producer: PubSubProducer)
{
    let bot_id = "60";

    let mut cached_players = get_players_for_bot(bot_id);
    let mut expiration_time = Utc::now().timestamp() + 5;  

    loop
    {
        let requests: Vec<(u64, String)> = consumer.listen()
            .unwrap();

        for (key, _) in requests
        {
            trace!("Received lobby request for key: {:?}", key);

            let current_time = Utc::now();
            if current_time.timestamp() > expiration_time
            {
                cached_players = match get_players_for_bot(bot_id)
                {
                    Ok(players) =>
                    {
                        expiration_time = current_time.timestamp() + 5;
                        Ok(players)
                    },
                    Err(_) =>
                    {
                        expiration_time = current_time.timestamp() + 2;
                        error!("Failed to update cache");
                        continue;
                    }
                };
            }

            if let Ok(players) = &cached_players
            {
                if let Err(_) = producer.send_to_topic(ID_BULK_STATS_REQUESTS_TOPIC, key, players)
                {
                    error!("failed to respond to lobby request");
                }
            }
        }
    }
}

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

fn get_players_for_bot(bot_id: &str) -> Result<HashMap<u8, Player>>
{
    let game_ids_url = "https://entgaming.net/forum/games_fast.php";

    // Get current game_id
    let mut games_response = reqwest::get(game_ids_url)?;
    if !games_response.status().is_success()
    {
        bail!("Bad status: {}", games_response.status());
    }

    let games_text = games_response.text()?;

    let id_lobby = get_lobby_for_bot(bot_id, &games_text)?;

    // Get players in that game
    let game_by_id_url = format!("https://entgaming.net/forum/slots_fast.php?id={}", id_lobby.game_id);
    let mut lobby_response = reqwest::get(&game_by_id_url)?;

    let lobby_text = lobby_response.text()?;

    let mut players: HashMap<u8, Player> = HashMap::new();
    let mut index = 0;

    let tr_and_slot = And(Name("td"), Class("slot"));

    let lobby_dom = Document::from(lobby_text.as_str());
    for tr in lobby_dom.find(Name("tr"))
    {
        let mut slots = tr.find(tr_and_slot);
 
        let name = match slots.next()
        {
            None => continue,
            Some(name) =>
            {
                if name.text() == "Empty"
                {
                    /* Empty slot */
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

        let realm = match slots.next()
        {
            None => continue,
            Some(realm) => realm.text(),
        };

        // let ping = slots.next();
        // let elo = slots.next();
        // let win_loss = slots.next();

        players.insert(index, Player::new(name, realm));
        index = index + 1;
    }

    trace!("Update cached lobby");
    Ok(players)
}

fn main() {
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init();

    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::perform_loopback_test(&broker_uris, KAFKA_GROUP)
        .expect("Kafka not initialized yet");

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_LOBBY_REQUESTS_TOPIC, KAFKA_GROUP)
        .unwrap();

    let lobby_requests_thread = thread::spawn(move || {
        lobby_request_handler(consumer, producer);
    });

    let _ = lobby_requests_thread.join();
}
