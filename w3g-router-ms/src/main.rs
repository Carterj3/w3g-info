#![feature(plugin, decl_macro, custom_derive, integer_atomics)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

extern crate chrono;

use chrono::Utc;

extern crate rocket; 
extern crate rocket_contrib;

use rocket::State;

use rocket_contrib::Json; 

extern crate triple_buffer;

use triple_buffer::{TripleBuffer, Output, Input};


extern crate w3g_common;

use w3g_common::errors::Result; 

use w3g_common::pubsub::model::{IdStats, Message, Player};
use w3g_common::pubsub::producer::PubSubProducer;
use w3g_common::pubsub::consumer::PubSubConsumer;
use w3g_common::pubsub::{ID_LOBBY_STATS_RESPONSE_TOPIC, ID_LEADER_BOARD_REQUEST_TOPIC, ID_LEADER_BOARD_RESPONSE_TOPIC};

use w3g_common::api::island_defense;

use std::collections::VecDeque;
use std::sync::Mutex; 
use std::thread;
use std::collections::HashMap;
use std::env;
use std::cmp::Ordering;

const KAFKA_GROUP: &'static str = "w3g-router-ms";

struct RustRouterConfig 
{
    id_lobby: Mutex<Output<island_defense::lobby::Lobby>>,
    id_leader_board: Mutex<Output<island_defense::leaderboard::LeaderBoard>>,
}


///
/// Gets the stats of all the players in the current island defense lobby
/// 
/// * `common` - the stored common configuration needed to do anything (i.e kafka parameters)
#[get("/lobby/island-defense")]
fn lobby_island_defense(common: State<RustRouterConfig>) -> Result<Json<island_defense::lobby::Lobby>>
{ 
    match common.id_lobby.lock()
    {
        Ok(mut id_lobby) => Ok(Json(id_lobby.read().clone())),
        Err(error) => bail!("Failed to acquire lock because {}", error),
    }
}

#[get("/leaderBoard/island-defense")]
fn leaderboard_island_defense(common: State<RustRouterConfig>) -> Result<Json<island_defense::leaderboard::LeaderBoard>>
{ 
    match common.id_leader_board.lock()
    {
        Ok(mut id_leader_board) => Ok(Json(id_leader_board.read().clone())),
        Err(error) => bail!("Failed to acquire lock because {}", error),
    }
}



fn id_leader_board_updater(mut consumer: PubSubConsumer, mut producer: PubSubProducer, mut buffer_input: Input<island_defense::leaderboard::LeaderBoard>)
{
    let mut expiration_time = Utc::now().timestamp();

    loop {
        let current_time = Utc::now().timestamp();
        if current_time > expiration_time
        {
            let debug = Some(HashMap::new());

            let mut destinations = VecDeque::with_capacity(1); 
            destinations.push_back(String::from(ID_LEADER_BOARD_RESPONSE_TOPIC));

            let message: Message<u32> = Message::new(10u32, destinations, debug); 

            if let Ok(_) = producer.send_to_topic(ID_LEADER_BOARD_REQUEST_TOPIC, 60u64, &message)
            {
                expiration_time = current_time + 5;
            }
        }

        let responses: Vec<(u64, Message<(HashMap<u32, IdStats>, HashMap<u32, IdStats>)>)> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Failed to parse leaderboard: {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if responses.is_empty()
        {
            thread::yield_now();
            continue;
        }

        for (key, message) in responses
        { 
            let (builders, titans) = message.data;
            
            let mut leaderboard = island_defense::leaderboard::LeaderBoard::new(
                        builders.into_iter().map(|(k, v)| island_defense::leaderboard::Builder::from_id_stats(v)).collect(),
                        titans.into_iter().map(|(k, v)| island_defense::leaderboard::Titan::from_id_stats(v)).collect());

            leaderboard.builders.sort_by(|left, right| right.rating.partial_cmp(&left.rating).unwrap_or(Ordering::Equal));
            leaderboard.titans.sort_by(|left, right| right.rating.partial_cmp(&left.rating).unwrap_or(Ordering::Equal));

            buffer_input.write(leaderboard);
        }
    }
}

fn id_lobby_updater(mut consumer: PubSubConsumer, mut producer: PubSubProducer, mut buffer_input: Input<island_defense::lobby::Lobby>)
{

    loop {

        let responses: Vec<(u64, Message<HashMap<u32, IdStats>>)> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Bad listen? {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if responses.is_empty()
        {
            thread::yield_now();
            continue;
        }

        for (key, message) in responses
        { 
            let players = message.data;
            match convert_map_to_lobby(players)
            {
                Err(error) => error!("failed to convert players to lobby because {}", error),
                Ok(lobby) => buffer_input.write(lobby),
            }
        }
    }
}


fn convert_map_to_lobby(mut players: HashMap<u32, IdStats>) -> Result<island_defense::lobby::Lobby>
{
    if players.len() > (std::u8::MAX as usize)
    {
        bail!("players.len(): {} must be smaller than u8's max: {}", players.len(), std::u8::MAX);
    }

    let mut builder_stats = HashMap::with_capacity(players.len());
    let mut builder_bbts = HashMap::with_capacity(players.len());
    let mut titan_stats = HashMap::with_capacity(1);
    let mut titan_bbts = HashMap::with_capacity(1);

    for slot in 0..10
    {
        if let Some(player) = players.remove(&slot)
        {
            builder_bbts.insert(slot, player.builder_stats.rating.clone()); 
            builder_stats.insert(slot, player);
        }
    }

    for slot in 10..11
    {
        if let Some(player) = players.remove(&slot)
        {
            titan_bbts.insert(slot, player.titan_stats.rating.clone());
            titan_stats.insert(slot, player);
        }
    }

    let ((builder_win_builder_ratings, builder_win_titan_ratings), (titan_win_builder_ratings, titan_win_titan_ratings)) = w3g_common::rating::compute_potential_ratings(&builder_bbts, &titan_bbts)?;

    let num_builders = builder_stats.len();
    let num_titans = titan_stats.len();

    let mut builders: Vec<island_defense::lobby::Builder> = Vec::with_capacity(num_builders);
    let mut titans: Vec<island_defense::lobby::Titan> = Vec::with_capacity(num_titans);

    let mut builders_rating = island_defense::lobby::Rating::new(0.0, 0.0, 0.0);
    let mut titans_rating = island_defense::lobby::Rating::new(0.0, 0.0, 0.0);

    for (slot, stat) in builder_stats.into_iter()
    {
        match ( builder_win_builder_ratings.get(&slot), titan_win_builder_ratings.get(&slot))
        {
            ( Some(win_rating), Some(loss_rating) ) =>
            {   
                let rating = stat.builder_stats.rating.mu();
                let rating = island_defense::lobby::Rating::new(rating, win_rating.mu() - rating, loss_rating.mu() - rating);

                builders_rating.mean_rating += rating.mean_rating / (num_builders as f64);
                builders_rating.potential_gain += rating.potential_gain / (num_builders as f64);
                builders_rating.potential_loss += rating.potential_loss / (num_builders as f64);

                builders.push(island_defense::lobby::Builder::new(slot as u8, stat.player.name, stat.player.realm, rating, stat.builder_stats.wins, stat.builder_stats.losses, stat.builder_stats.ties));
            },
            _ => bail!("Builder slot: {} did not have ratings computed", slot),
        };
    } 

    for (slot, stat) in titan_stats.into_iter()
    {
        match ( titan_win_titan_ratings.get(&slot), builder_win_titan_ratings.get(&slot))
        {
            ( Some(win_rating), Some(loss_rating) ) =>
            {   
                let rating = stat.titan_stats.rating.mu();
                let rating = island_defense::lobby::Rating::new(rating, win_rating.mu() - rating, loss_rating.mu() - rating);

                titans_rating.mean_rating += rating.mean_rating / (num_builders as f64);
                titans_rating.potential_gain += rating.potential_gain / (num_builders as f64);
                titans_rating.potential_loss += rating.potential_loss / (num_builders as f64);

                titans.push(island_defense::lobby::Titan::new(slot as u8, stat.player.name, stat.player.realm, rating, stat.titan_stats.wins, stat.titan_stats.losses, stat.titan_stats.ties));
            },
            _ => bail!("Titan slot: {} did not have ratings computed", slot),
        };
    } 


    Ok(island_defense::lobby::Lobby::new(island_defense::lobby::BuilderTeam::new(builders, builders_rating), island_defense::lobby::TitanTeam::new(titans, titans_rating)))
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
    w3g_common::pubsub::delay_until_kafka_ready(&broker_uris, KAFKA_GROUP);
   

    let empty_lobby = island_defense::lobby::Lobby::new(island_defense::lobby::BuilderTeam::new(Vec::new(), island_defense::lobby::Rating::new(0.0, 0.0, 0.0)), island_defense::lobby::TitanTeam::new(Vec::new(), island_defense::lobby::Rating::new(0.0, 0.0, 0.0)));
    let (lobby_input, lobby_output) = TripleBuffer::new(empty_lobby).split();

    let empty_leaderboard = island_defense::leaderboard::LeaderBoard::new(Vec::new(), Vec::new());
    let (leader_board_input, leader_board_output) = TripleBuffer::new(empty_leaderboard).split();

    let router_config = RustRouterConfig {
        id_lobby: Mutex::new(lobby_output),
        id_leader_board: Mutex::new(leader_board_output),
    };

    // Lobby
    let producer = PubSubProducer::new(broker_uris.clone())
             .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_LOBBY_STATS_RESPONSE_TOPIC, KAFKA_GROUP)
        .unwrap();

    let id_lobby_updater_thread = thread::spawn(move || {
        id_lobby_updater(consumer, producer, lobby_input);
    });

    // Leaderboard
    let producer = PubSubProducer::new(broker_uris.clone())
             .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_LEADER_BOARD_RESPONSE_TOPIC, KAFKA_GROUP)
        .unwrap();

    let id_leader_board_updater_thread = thread::spawn(move || {
        id_leader_board_updater(consumer, producer, leader_board_input);
    });

    rocket::ignite()
        .mount("v1", routes![lobby_island_defense, leaderboard_island_defense])
        .manage(router_config)
        .launch();

    let _ = id_lobby_updater_thread.join();
    let _ = id_leader_board_updater_thread.join();
}
