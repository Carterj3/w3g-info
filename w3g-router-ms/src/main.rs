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

use w3g_common::pubsub::PubSubProducer;
use w3g_common::pubsub::PubSubConsumer;
use w3g_common::pubsub::model::{IdGameResult, Player, IdTeam}; 
use w3g_common::pubsub::W3G_LOOPBACK_TOPIC;
use w3g_common::pubsub::{ID_BULK_STATS_RESPONSES_TOPIC, ID_BULK_STATS_REQUESTS_TOPIC, ID_LOBBY_REQUESTS_TOPIC, ID_STATS_UPDATES_TOPIC, ID_STATS_LEADERBOARD_REQUEST_TOPIC, ID_STATS_LEADERBOARD_RESPONSE_TOPIC};

use w3g_common::api::island_defense;

use std::sync::Mutex; 
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::env;

const KAFKA_GROUP: &'static str = "w3g-router-ms";

struct RustRouterConfig 
{
    id_lobby: Mutex<Output<island_defense::lobby::Lobby>>,
    id_leader_board: Mutex<Output<island_defense::leaderboard::LeaderBoard>>,
}

#[get("/")]
fn index() -> Result<Json<Vec<(u64, String)>>> {

    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };

    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let mut consumer = PubSubConsumer::new(broker_uris.clone(), W3G_LOOPBACK_TOPIC, KAFKA_GROUP)
        .unwrap();

    producer.send_to_topic(W3G_LOOPBACK_TOPIC, 1337, "Hello World")?;
    let responses: Vec<(u64, String)> = consumer.listen()?;

    Ok(Json(responses))
}

#[get("/leaderBoard")]
fn leader_board_test(common: State<RustRouterConfig>) -> Result<Json<island_defense::leaderboard::LeaderBoard>>
{
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();

    producer.send_to_topic(ID_STATS_LEADERBOARD_REQUEST_TOPIC, Utc::now().timestamp_nanos() as u64, 10i64)?;

    thread::sleep(Duration::from_millis(50));

    match common.id_leader_board.lock()
    {
        Ok(mut id_leader_board) => Ok(Json(id_leader_board.read().clone())),
        Err(error) => bail!("Failed to acquire lock because {}", error),
    }
}

#[get("/ratings")]
fn ratings_test(common: State<RustRouterConfig>) -> Result<Json<island_defense::lobby::Lobby>>
{
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();

    let game1 = IdGameResult::new(  vec![Player::new("Builder1", "Test"), Player::new("Builder2", "Test")]
                                 ,  vec![Player::new("Titan1", "Test")]
                                 ,  IdTeam::Titan);
    let game2 = IdGameResult::new(  vec![Player::new("builder1", "Test"), Player::new("builder2", "Test")]
                                 ,  vec![Player::new("titan1", "Test")]
                                 ,  IdTeam::Builder);

    producer.send_to_topic(ID_STATS_UPDATES_TOPIC, Utc::now().timestamp_nanos() as u64, game1)?;
    producer.send_to_topic(ID_STATS_UPDATES_TOPIC, Utc::now().timestamp_nanos() as u64, game2)?;

    thread::sleep(Duration::from_millis(25));

    let mut lobby = HashMap::new();
    lobby.insert(0, Player::new("Builder1", "Test"));
    lobby.insert(1, Player::new("builder2", "Test"));
    lobby.insert(2, Player::new("titan1", "Test"));
    lobby.insert(3, Player::new("Titan1", "Test"));

    producer.send_to_topic(ID_BULK_STATS_REQUESTS_TOPIC, Utc::now().timestamp_nanos() as u64, lobby)?;

    thread::sleep(Duration::from_millis(50));

    match common.id_lobby.lock()
    {
        Ok(mut id_lobby) => Ok(Json(id_lobby.read().clone())),
        Err(error) => bail!("Failed to acquire lock because {}", error),
    }
}

#[get("/log")]
fn log_test() -> &'static str {

    trace!("This is a trace message");
    debug!("This is a debug message");
    info!("This is an info message");
    warn!("This is a warn message");
    error!("This is an error message");

    "See logs?"
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
            if let Ok(_) = producer.send_to_topic(ID_STATS_LEADERBOARD_REQUEST_TOPIC, 0, 10i64)
            {
                expiration_time = current_time + 5;
            }
        }

        let leaderboards: Vec<(u64, island_defense::leaderboard::LeaderBoard)> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Failed to parse leaderboard: {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if leaderboards.is_empty()
        {
            thread::yield_now();
            continue;
        }

        for (key, leaderboard) in leaderboards
        { 
            trace!("Received leaderboard response for key: {:?} = {:?}", key, leaderboard);

            buffer_input.write(leaderboard);
        }
    }
}

fn id_lobby_updater(mut consumer: PubSubConsumer, mut producer: PubSubProducer, mut buffer_input: Input<island_defense::lobby::Lobby>)
{
    let mut expiration_time = Utc::now().timestamp();

    loop {
        let current_time = Utc::now().timestamp();
        if current_time > expiration_time
        {
            if let Ok(_) = producer.send_to_topic(ID_LOBBY_REQUESTS_TOPIC, 0, "")
            {
                expiration_time = current_time + 5;
            }
        }

        let bulk_stats: Vec<(u64, island_defense::lobby::Lobby)> = match consumer.listen()
        {
            Err(_) =>
            {
                error!("Bad listen?");
                continue;
            },
            Ok(data) => data,
        };

        if bulk_stats.is_empty()
        {
            thread::yield_now();
            continue;
        }

        for (key, bulk_stat) in bulk_stats
        { 
            trace!("Received lobby response for key: {:?} = {:?}", key, bulk_stat);

            buffer_input.write(bulk_stat);
        }
    }
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
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_BULK_STATS_RESPONSES_TOPIC, KAFKA_GROUP)
        .unwrap();

    let id_lobby_updater_thread = thread::spawn(move || {
        id_lobby_updater(consumer, producer, lobby_input);
    });

    // Leaderboard
    let producer = PubSubProducer::new(broker_uris.clone())
             .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_STATS_LEADERBOARD_RESPONSE_TOPIC, KAFKA_GROUP)
        .unwrap();

    let id_leader_board_updater_thread = thread::spawn(move || {
        id_leader_board_updater(consumer, producer, leader_board_input);
    });

    rocket::ignite()
        .mount("v1", routes![index, log_test, ratings_test, leader_board_test, lobby_island_defense, leaderboard_island_defense])
        .manage(router_config)
        .launch();

    let _ = id_lobby_updater_thread.join();
    let _ = id_leader_board_updater_thread.join();
}
