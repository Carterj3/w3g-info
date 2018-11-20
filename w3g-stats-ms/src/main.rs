#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

#[macro_use]
extern crate error_chain;

extern crate mongodb;
extern crate bson;

use mongodb::coll::Collection;
use mongodb::coll::results::UpdateResult;
use mongodb::coll::options::{UpdateOptions, FindOptions, IndexOptions};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase; 

use bson::{Bson, Document};

extern crate bbt; 
use bbt::Rater;

extern crate regex;

extern crate w3g_common; 

use w3g_common::pubsub::consumer::PubSubConsumer;

use w3g_common::pubsub::producer::PubSubProducer;

use w3g_common::pubsub::model::Message;
use w3g_common::pubsub::model::Player;
use w3g_common::pubsub::model::IdStats; 
use w3g_common::pubsub::model::IdTeam;
use w3g_common::pubsub::model::IdGameResult;

use w3g_common::pubsub::{ID_LEADER_BOARD_REQUEST_TOPIC, ID_STATS_REQUEST_TOPIC, ID_GAME_RESULT_TOPIC};

use w3g_common::errors::Result;

use std::collections::HashMap; 
use std::env;

const KAFKA_GROUP: &'static str = "id-stats-ms";

fn create_case_insensitive_filter(player: &Player) -> Document
{
    let mut filter = Document::new();
    filter.insert("player.name", Bson::RegExp(format!("^{}$", regex::escape(&player.name)), "i".to_string()));
    filter.insert("player.realm", Bson::String(player.realm.clone()));

    filter
}

fn find_player_stats(player: &Player, collection: &Collection) -> IdStats
{
    let filter = create_case_insensitive_filter(player);

    match collection.find_one(Some(filter), None)
    { 
        Ok(Some(stats)) =>
        {
            match bson::from_bson(Bson::Document(stats))
            {
                Ok(stats) => stats,
                Err(error) => 
                {
                    error!("failed to parse bson for player: {:?} because {}", player, error);
                    IdStats::default(player.clone())
                },
            }
        },
        Ok(None) => IdStats::default(player.clone()),
        Err(error) => 
        {
            error!("Failed to find player: {:?} because {}", player, error);
            IdStats::default(player.clone())
        },
    }
}

fn update_player_stats(stats: &IdStats, collection: &Collection) -> Result<UpdateResult>
{
    match bson::to_bson(stats)?
    {
        Bson::Document(document) => {
            let filter = create_case_insensitive_filter(&stats.player);

            let mut options = UpdateOptions::new();
            options.upsert = Some(true);

            Ok(collection.replace_one(filter, document, Some(options))?)
        },
        _ => bail!("Error, BSON was not converted to Document"),
    }
}

fn update_ratings(builders: &Vec<IdStats>, titans: &Vec<IdStats>, outcome: &IdTeam) -> Result<(Vec<bbt::Rating>, Vec<bbt::Rating>)>
{
    let rater = Rater::new(1500.0 / 6.0);

    // Ratings are all screwy with unequal sized teams so make them same size
    let lcm = builders.len() * titans.len();

    let mut builder_ratings = Vec::new();
    let mut titan_ratings = Vec::new();
    for i in 0..lcm
    {
        builder_ratings.push(builders.get(i % builders.len())
            .ok_or(format!("Bad index: {} of {}", i, builders.len()))?
            .builder_stats.rating
            .clone());
        titan_ratings.push(titans.get(i % titans.len())
            .ok_or(format!("Bad index: {} of {}", i, titans.len()))?
            .titan_stats.rating
            .clone());
    }

    let mut new_ratings = match outcome
    {
        IdTeam::Builder => rater.update_ratings(vec!(builder_ratings, titan_ratings), vec!(1, 2))?.into_iter(),
        IdTeam::Titan => rater.update_ratings(vec!(builder_ratings, titan_ratings), vec!(2, 1))?.into_iter(),
        /* Ties don't cause stats changes otherwise a stalling titan can actually gain ELO */
        IdTeam::Tie => vec!(builder_ratings, titan_ratings).into_iter(),
    };

    let mut builder_ratings = new_ratings.next().ok_or("None found for builder_ratings")?;
    builder_ratings.truncate(builders.len());
    let mut titan_ratings = new_ratings.next().ok_or("None found for titan_ratings")?;
    titan_ratings.truncate(titans.len());

    Ok((builder_ratings, titan_ratings))
}

fn update_stats(result: IdGameResult, collection: &Collection) -> Result<()>
{
    let (titan_wins, titan_ties, titan_losses, builder_wins, builder_ties, builder_losses) = match result.winner
    {
        IdTeam::Builder => (0, 0, 1, 1, 0, 0),
        IdTeam::Titan   => (1, 0, 0, 0, 0, 1),
        IdTeam::Tie     => (0, 1, 0, 0, 1, 0),
    };

    let mut builder_stats = Vec::new();
    let mut titan_stats = Vec::new();

    for builder in result.builders
    {
        builder_stats.push(find_player_stats(&builder, collection));
    }
    for titan in result.titans
    {
        titan_stats.push(find_player_stats(&titan, collection));
    }

    let builder_stats_len = builder_stats.len();
    let (builder_ratings, titan_ratings) = update_ratings(&builder_stats, &titan_stats, &result.winner)?;

    for i in 0..builder_stats.len()
    {
        match (builder_stats.get_mut(i), builder_ratings.get(i))
        {
            (Some(stats), Some(new_rating)) =>
            {
                stats.builder_stats.rating = new_rating.clone();
                stats.builder_stats.wins += builder_wins;
                stats.builder_stats.losses += builder_losses;
                stats.builder_stats.ties += builder_ties;

                match update_player_stats(&stats, &collection)
                {
                    Ok(_) => trace!("Updated stats for player: {:?}", stats.player),
                    Err(error) => error!("Failed to update stats for player: {:?} because {}", stats.player, error),
                }
            },
            _ => error!("Index: {} was outside of stats: {} or ratings: {}", i, builder_stats_len, builder_ratings.len()),
        }
    }

    let titan_stats_len = titan_stats.len();
    for i in 0..titan_stats.len()
    {
        match (titan_stats.get_mut(i), titan_ratings.get(i))
        {
            (Some(stats), Some(new_rating)) =>
            {
                stats.titan_stats.rating = new_rating.clone();
                stats.titan_stats.wins += titan_wins;
                stats.titan_stats.losses += titan_losses;
                stats.titan_stats.ties += titan_ties;

                match update_player_stats(&stats, &collection)
                {
                    Ok(_) => trace!("Updated stats for player: {:?}", stats.player),
                    Err(error) => error!("Failed to update stats for player: {:?} because {}", stats.player, error),
                }
            },
            _ => error!("Index: {} was outside of stats: {} or ratings: {}", i, titan_stats_len, builder_ratings.len()),
        }
    }

    Ok(())
}

fn get_top_players(size: u32, sort: Document, collection: &Collection) -> Result<HashMap<u32, IdStats>>
{
    let mut options = FindOptions::new(); 
    options.limit = Some(size as i64);
    options.sort = Some(sort);

    let mut players: HashMap<u32, IdStats> = HashMap::with_capacity(size as usize);
    let mut cursor = collection.find(None, Some(options))?;

    while players.len() < (size as usize) &&  cursor.has_next()?
    {
        for player in cursor.drain_current_batch()?
                            .into_iter()
                            .filter_map(|document| bson::from_bson(Bson::Document(document)).ok())
                            .collect::<Vec<IdStats>>()
        {
            if ! (players.len() < (size as usize))
            {
                trace!("Almost increased players map beyond requested size");
                break;
            }

            let rank = players.len() as u32;
            players.insert(rank, player);
        }
    }
        
    Ok(players)
}

  
 

fn leaderboard_handler(collection: &Collection, consumer: &mut PubSubConsumer, producer: &mut PubSubProducer) -> Result<()>
{
    let messages: Vec<(u64, Message<u32>)> = consumer.listen()?;

    for (key, mut message) in messages.into_iter()
    {
        match message.destinations.pop_front()
        {
            None => {},
            Some(topic) =>
            {
                let size = message.data;
                let debug = message.debug;

                let mut builders_sort = Document::new();
                builders_sort.insert("builder_stats.rating.mu", Bson::I32(-1));
                let builders = get_top_players(size, builders_sort, collection)?;

                let mut titans_sort = Document::new();
                titans_sort.insert("titan_stats.rating.mu", Bson::I32(-1));
                let titans = get_top_players(size, titans_sort, collection)?;
 
                let response: Message<(HashMap<u32, IdStats>, HashMap<u32, IdStats>)> = Message::new((builders, titans), message.destinations, debug);

                producer.send_to_topic(&topic, key, &response)?;
            }
        }  
    }

    Ok(())
}

fn lobby_handler(collection: &Collection, consumer: &mut PubSubConsumer, producer: &mut PubSubProducer) -> Result<()>
{
    let messages: Vec<(u64, Message<HashMap<u32, Player>>)> = consumer.listen()?;

    for (key, mut message) in messages.into_iter()
    {
        match message.destinations.pop_front()
        {
            None => {},
            Some(topic) =>
            {
                let players = message.data;
                let debug = message.debug;

                let mut stats = HashMap::with_capacity(players.len());
                for (key, player) in players.into_iter()
                {
                    stats.insert(key, find_player_stats(&player, collection));
                }
 
                let response: Message<HashMap<u32, IdStats>> = Message::new(stats, message.destinations, debug);

                producer.send_to_topic(&topic, key, &response)?;
            }
        }
    }  
    Ok(())
}

fn stats_update_handler(collection: &Collection, consumer: &mut PubSubConsumer, producer: &mut PubSubProducer) -> Result<()>
{
    let messages: Vec<(u64, Message<IdGameResult>)> = consumer.listen()?;

    for (key, message) in messages.into_iter()
    {
        update_stats(message.data, collection)?;
    }
    
    Ok(())
}

fn main() {
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init();

    // Kafka set-up
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::delay_until_kafka_ready(&broker_uris, KAFKA_GROUP);

    // Mongo set-up
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
        .unwrap_or(String::from("player-stats"));

    let client = Client::connect(&mongo_host, mongo_port)
        .unwrap();
    let database = client.db(&mongo_db);

    // Create Index
    let _ = database.create_collection(&mongo_collecion, None);
    let collection = database.collection(&mongo_collecion);
    
    collection.drop_indexes()
        .unwrap();

    let mut builder_options = IndexOptions::new();
    builder_options.name = Some(String::from("builders_rating"));
    builder_options.unique = Some(false);
    builder_options.background = Some(false);

    let mut builders_rating_index = Document::new();
    builders_rating_index.insert("builder_stats.rating.mu", Bson::I32(-1));
    collection.create_index(builders_rating_index, Some(builder_options))
        .unwrap();

    let mut titan_options = IndexOptions::new();
    titan_options.name = Some(String::from("titans_rating"));
    titan_options.unique = Some(false);
    titan_options.background = Some(false);

    let mut titans_rating_index = Document::new();
    titans_rating_index.insert("titan_stats.rating.mu", Bson::I32(-1));
    collection.create_index(titans_rating_index, Some(titan_options))
        .unwrap();

    

    // Request loop
    let collection = database.collection(&mongo_collecion);
   
    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let mut leaderboard_consumer = PubSubConsumer::new(broker_uris.clone(), ID_LEADER_BOARD_REQUEST_TOPIC, KAFKA_GROUP)
        .unwrap();
    let mut lobby_consumer = PubSubConsumer::new(broker_uris.clone(), ID_STATS_REQUEST_TOPIC, KAFKA_GROUP)
        .unwrap();
    let mut stats_update_consumer = PubSubConsumer::new(broker_uris.clone(), ID_GAME_RESULT_TOPIC, KAFKA_GROUP)
        .unwrap();

    loop {
        match leaderboard_handler(&collection, &mut leaderboard_consumer, &mut producer)
        {
            Ok(_) => {},
            Err(error) => error!("Failed to handle leadboard request because {}", error),
        }

        match lobby_handler(&collection, &mut lobby_consumer, &mut producer)
        {
            Ok(_) => {},
            Err(error) => error!("Failed to handle lobby request because {}", error),
        }

        match stats_update_handler(&collection, &mut stats_update_consumer, &mut producer)
        {
            Ok(_) => {},
            Err(error) => error!("Failed to handle stats update request because {}", error),
        }
    }
    
}
