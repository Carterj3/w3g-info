extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

extern crate mongodb;
#[macro_use] extern crate bson;

use mongodb::coll::Collection;
use mongodb::coll::options::FindOptions;
use mongodb::ThreadedClient;
use mongodb::Client; 
use mongodb::db::ThreadedDatabase; 

use bson::Bson::Document as BsonDocument;

extern crate reqwest; 
extern crate select;

use select::predicate::{And, Class, Name};
use select::document::Document as HtmlDocument;

#[macro_use]
extern crate serde_derive; 
extern crate serde_json;

#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

extern crate w3g_common;

use w3g_common::errors::Result;
use w3g_common::pubsub::model::Player;
use w3g_common::pubsub::producer::PubSubProducer;
use w3g_common::pubsub::ID_REPLAY_RESPONSES_TOPIC;
use w3g_common::parser::Replay;


use std::env; 
use std::thread;
use std::fs::File;
use std::cmp::{min, max};
use std::time::Duration;
use std::io::{Write, Cursor};
use std::path::PathBuf;

const KAFKA_GROUP: &'static str = "w3g-lobby-ms";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct GameIdDto
{
    /// Use i64 instead of u64 because BSON cannot store unsigned
    game_id: i64,
    was_parsed: bool,
    was_sent_over_pubsub: bool,
}

impl GameIdDto
{
    fn new(game_id: i64, was_parsed: bool, was_sent_over_pubsub: bool) -> GameIdDto
    {
        GameIdDto {
            game_id,
            was_parsed,
            was_sent_over_pubsub,
        }
    }
}

 /// page_number ~ [1, 250]
 fn get_game_ids(page_number: u8) -> Result<Vec<i64>>
 {
     if page_number < 1 || page_number > 250
     {
         panic!("Invalid page_number: {}. Should be [1, 250]", page_number);
     }

    let games_url = format!("https://entgaming.net/customstats/islanddefense/games/{}/", page_number);
    let mut games_response = reqwest::get(&games_url)?;

    let games_text = games_response.text()?;

    let mut game_ids = Vec::new();

    lazy_static! {
        static ref GAME_ID_REGEX: Regex = Regex::new(r"https://entgaming.net/customstats/islanddefense/game/(?P<id>\d+)/").unwrap();
    }
 
    for capture in GAME_ID_REGEX.captures_iter(games_text.as_str())
    {
        if let Some(game_id) = capture.name("id")
        {
            if let Ok(game_id) = game_id.as_str().parse::<i64>()
            {
                game_ids.push(game_id);
            }
        }
    }
 
    Ok(game_ids)
 }

/// game_id -> Vec<Player>, replay_url
 fn get_game(replay_path: &Option<PathBuf>, game_id: i64) -> Result<(Vec<Player>, Replay)>
 {
    let game_url = format!("https://entgaming.net/customstats/islanddefense/game/{}/", game_id);
    let mut games_response = reqwest::get(&game_url)?;

    let mut games_bytes: Vec<u8> = Vec::new();
    games_response.copy_to(&mut games_bytes)?;

    if let Some(replay_path) = replay_path {
        let mut path = replay_path.clone();
        path.push(format!("{}.html", game_id));

        trace!("Saving html to path: {:?}", path);
        let mut file = File::create(path)?;
        file.write_all(&games_bytes)?;
        file.sync_all()?
    }
 
    let games_cursor = Cursor::new(games_bytes);

    let selector = And(Name("tr"), Class("GameRow"));
    lazy_static! {
        static ref NAME_REALM_REGEX: Regex = Regex::new(r#"<a href="https://entgaming.net/customstats/islanddefense/player/(?P<name>.*?)/">(?:.*)</a><br>(?P<realm>.*?)</td>"#).unwrap();
    }
 
    let mut players = Vec::new();

    let games_dom = HtmlDocument::from_read(games_cursor)?;
    for tr in games_dom.find(selector)
    {
        let mut tds = tr.find(Name("td"));

        let _hero_icon_td = tds.next();
        if let Some(player_td) = tds.next()
        {
            for capture in NAME_REALM_REGEX.captures_iter(&player_td.html())
            {
                if let (Some(name), Some(realm)) = (capture.name("name"), capture.name("realm"))
                {
                    players.push(Player::new(String::from(name.as_str()), String::from(realm.as_str())));
                }
            }
        } 
    }

    let replay_url = format!("http://storage.entgaming.net/replay/download.php?f={game_id}.w3g&fc={game_id}.w3g", game_id=game_id);
    let mut replay_response = reqwest::get(&replay_url)?;

    /* Need to use .copy_to() as .text() has incorect length and bad bytes */
    let mut replay_bytes: Vec<u8> = Vec::new();
    replay_response.copy_to(&mut replay_bytes)?;

    if let Some(replay_path) = replay_path {
        let mut path = replay_path.clone();
        path.push(format!("{}.w3g", game_id));

        trace!("Saving w3g to path: {:?}", path);
        let mut file = File::create(path)?;
        file.write_all(&replay_bytes)?;
        file.sync_all()?
    }
 
    let mut replay_cursor = Cursor::new(replay_bytes);
    Ok((players, w3g_common::parser::parse_replay(&mut replay_cursor)?))
 }

fn store_game_id(dto: GameIdDto, collection: &Collection)
{
    match bson::to_bson(&dto)
    {
        Ok(BsonDocument(document)) =>
        {
            match collection.insert_one(document, None)
            {
                Err(error) => error!("Error, unable to save updates to player: {}", error),
                Ok(_) => (),
            }
        },
        Ok(_) => error!("Error, BSON was not converted to Document"),
        Err(error) => error!("Error, unable to convert the BSON object: {}", error),
    }
}

fn find_maximum_game_id(collection: &Collection) -> Result<i64>
{
    // https://stackoverflow.com/a/32077240/1991577
    let filter = doc!{
        "was_sent_over_pubsub" => true
    };
    let mut options = FindOptions::new();
    options.limit = Some(1);
    options.sort = Some(
        doc!{
            "game_id" => -1,
        }
    );

    match collection.find_one(Some(filter), Some(options))?
    {
        None => Ok(std::i64::MIN),
        Some(doc) =>
        {
            match bson::from_bson::<GameIdDto>(BsonDocument(doc))
            {
                Ok(dto) => Ok(dto.game_id),
                Err(error) =>
                {
                    warn!("failed to convert bson to GameIdDto. {}", error);
                    Ok(std::i64::MIN)
                }
            }
        },
    }
}

fn find_minimum_game_id(collection: &Collection) -> Result<i64>
{
    // https://stackoverflow.com/a/32077240/1991577
    let filter = doc!{
        "was_sent_over_pubsub" => true
    };
    let mut options = FindOptions::new();
    options.limit = Some(1);
    options.sort = Some(
        doc!{
            "game_id" => 1,
        }
    );

    match collection.find_one(Some(filter), Some(options))?
    {
        None => Ok(std::i64::MAX),
        Some(doc) =>
        {
            match bson::from_bson::<GameIdDto>(BsonDocument(doc))
            {
                Ok(dto) => Ok(dto.game_id),
                Err(_) => Ok(std::i64::MAX),
            }
        },
    }
}

fn download_replays(replay_path: &Option<PathBuf>, mut producer: PubSubProducer, collection: Collection)
{
    let mut min_game_id = find_minimum_game_id(&collection)
        .unwrap();
    let mut max_game_id = find_maximum_game_id(&collection)
        .unwrap();

    loop {
        info!("download_replays loop starting with Min: {}, Max: {}", min_game_id, max_game_id);

        for page_number in 1..=250
        {
            trace!("asking for page: {}. Min: {}, Max: {}", page_number, min_game_id, max_game_id);
            if let Ok(game_ids) = get_game_ids(page_number)
            {
                let new_game_ids: Vec<i64> = game_ids.into_iter()
                        .filter(|game_id| *game_id < min_game_id || *game_id > max_game_id)
                        .collect();
                
                if new_game_ids.is_empty()
                {
                    thread::yield_now();
                    break;
                }

                for game_id in new_game_ids
                {
                    min_game_id = min(min_game_id, game_id);
                    max_game_id = max(max_game_id, game_id);

                    match get_game(replay_path, game_id)
                    {
                        Ok((players, replay)) =>
                        {
                            debug!("parsed id: {} with {} players", game_id, players.len()); 

                            match producer.send_to_topic(ID_REPLAY_RESPONSES_TOPIC, game_id as u64, (players, replay))
                            {
                                Ok(_) => {
                                    trace!("sent out id: {}", game_id);
                                    store_game_id(GameIdDto::new(game_id, true, true), &collection)
                                },
                                Err(error) =>
                                {
                                    error!("failed to send id: {} because {}", game_id, error); 
                                    store_game_id(GameIdDto::new(game_id, true, false), &collection)
                                },
                            }
                        },
                        Err(error) =>
                        {
                            error!("failed to handle id: {} because {}", game_id, error);
                            store_game_id(GameIdDto::new(game_id, false, false), &collection);
                        },
                    } 
                }
            }
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    /* Logger */
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init();

    /* Mongo */
    let mongo_host = env::var("MONGO_HOST")
        .unwrap_or(String::from("localhost"));

    let mongo_port = match env::var("MONGO_PORT")
    {
        Ok(port) => port.parse::<u16>().unwrap_or(27017),
        Err(_) => 27017,
    };
    
    let mongo_db = env::var("MONGO_DB")
        .unwrap_or(String::from("island-defense"));
    let mongo_collecion = env::var("MONGO_COLLECTION")
        .unwrap_or(String::from("replays"));

    let client = Client::connect(&mongo_host, mongo_port)
        .unwrap();
    let database = client.db(&mongo_db);

    let replay_path = env::var("REPLAY_PATH")
                        .map(|path| PathBuf::from(path))
                        .ok();

    if let Some(replay_path) = &replay_path
    {
        std::fs::create_dir_all(replay_path.clone())
            .unwrap();
    }

    // TODO: Create index on replays for `game_id`
    let collection = database.collection(&mongo_collecion); 

    /* Kafka */
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::perform_loopback_test(&broker_uris, KAFKA_GROUP)
        .expect("Kafka not initialized yet");

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();

     let download_replays_thread = thread::spawn(move || {
        download_replays(&replay_path, producer, collection);
    });

    let _ = download_replays_thread.join();
}

