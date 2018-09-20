#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

#[macro_use]
extern crate error_chain;

extern crate mongodb;
#[macro_use] extern crate bson;

use mongodb::coll::Collection;
use mongodb::coll::results::UpdateResult;
use mongodb::coll::options::UpdateOptions;
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase; 

use bson::Bson::Document;

extern crate bbt; 
use bbt::Rater;

extern crate w3g_common; 
 
use w3g_common::pubsub::PubSubConsumer;

use w3g_common::pubsub::PubSubProducer;

use w3g_common::pubsub::model::Player;
use w3g_common::pubsub::model::IdStats; 
use w3g_common::pubsub::model::IdTeam;
use w3g_common::pubsub::model::IdGameResult;

use w3g_common::pubsub::{ID_BULK_STATS_RESPONSES_TOPIC, ID_STATS_UPDATES_TOPIC, ID_BULK_STATS_REQUESTS_TOPIC};

use w3g_common::errors::Result;

use std::collections::HashMap;
use std::thread;
use std::env;

const KAFKA_GROUP: &'static str = "id-stats-ms";

fn find_player_stats(player: Player, collection: &Collection) -> IdStats
{
    let filter = doc!{
        "player.name" => player.name.clone(),
        "player.realm" => player.realm.clone(),
    };

    match collection.find_one(Some(filter), None)
    { 
        Ok(Some(stats)) =>
        {
            match bson::from_bson(Document(stats))
            {
                Ok(stats) => stats,
                Err(_) => IdStats::default(player),
            }
        },
        Ok(None) => IdStats::default(player),
        Err(_) => IdStats::default(player),
    }
}

fn update_player_stats(stats: &IdStats, collection: &Collection) -> Result<UpdateResult>
{
    match bson::to_bson(stats)?
    {
        Document(document) => {
            let filter = doc!{
                "player.name" => stats.player.name.clone(),
                "player.realm" => stats.player.realm.clone(),
            };
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
            .rating
            .clone());
        titan_ratings.push(titans.get(i % titans.len())
            .ok_or(format!("Bad index: {} of {}", i, titans.len()))?
            .rating
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

fn stats_update_handler(mut consumer: PubSubConsumer, collection: Collection)
{
    loop {
        let games: Vec<(u64, IdGameResult)> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Unable to listen because: {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if games.is_empty()
        {
            thread::yield_now();
        }

        for (game_id, result) in games
        {
            info!("Updating stats in game: {} per: {:?}", game_id, result);
            
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
                builder_stats.push(find_player_stats(builder, &collection));
            }
            for titan in result.titans
            {
                titan_stats.push(find_player_stats(titan, &collection));
            }

            let builder_stats_len = builder_stats.len();
            match update_ratings(&builder_stats, &titan_stats, &result.winner)
            {
                Err(error) => error!("Unable to updating ratings: {}", error),
                Ok((builder_ratings, titan_ratings)) =>
                {
                    for i in 0..builder_stats.len()
                    {
                        match (builder_stats.get_mut(i), builder_ratings.get(i))
                        {
                            (Some(stats), Some(new_rating)) =>
                            {
                                stats.rating = new_rating.clone();
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
                                stats.rating = new_rating.clone();
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
                }
            }
        }
    }
}

fn stats_bulk_request_handler(mut consumer: PubSubConsumer, mut producer: PubSubProducer, collection: Collection)
{
    loop 
    {
        let lobbies: Vec<(u64, HashMap<u8, Player>)> = match consumer.listen()
        {
            Err(_) =>
            {
                error!("Error?");
                continue;
            },
            Ok(data) => data,
        };

        if lobbies.is_empty()
        {
            thread::yield_now();
        }

        for (key, lobby) in lobbies
        {
            let mut lobby_stats = HashMap::new();
            for (slot, player) in lobby
            {
                lobby_stats.insert(slot, find_player_stats(player, &collection));
            }

            match producer.send_to_topic(ID_BULK_STATS_RESPONSES_TOPIC, key, lobby_stats)
            {
                Err(_) => error!("Error?"),
                _ => (),
            }
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
    // TODO: Create index on player-stats for `player.name`+`player.realm`


    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::perform_loopback_test(&broker_uris, KAFKA_GROUP)
        .expect("Kafka not initialized yet");

    // Bulk Stats Requests
    let collection = database.collection(&collection_name);
   
    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_BULK_STATS_REQUESTS_TOPIC, KAFKA_GROUP)
        .unwrap();

    let bulk_requests_thread = thread::spawn(move || {
        stats_bulk_request_handler(consumer, producer, collection);
    });

    // Stats Updates
    let collection = database.collection(&collection_name);

    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_STATS_UPDATES_TOPIC, KAFKA_GROUP)
        .unwrap();

    let updates_thread = thread::spawn(move || {
        stats_update_handler(consumer, collection);
    });

    let _ = bulk_requests_thread.join(); 
    let _ = updates_thread.join();
    
}
