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
use mongodb::coll::options::{UpdateOptions, FindOptions};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase; 

use bson::{Bson, Document};

extern crate bbt; 
use bbt::Rater;

extern crate regex;

extern crate w3g_common; 
 
use w3g_common::api::island_defense;

use w3g_common::pubsub::PubSubConsumer;

use w3g_common::pubsub::PubSubProducer;

use w3g_common::pubsub::model::Player;
use w3g_common::pubsub::model::IdStats; 
use w3g_common::pubsub::model::IdTeam;
use w3g_common::pubsub::model::IdGameResult;

use w3g_common::pubsub::{ID_BULK_STATS_RESPONSES_TOPIC, ID_STATS_UPDATES_TOPIC, ID_BULK_STATS_REQUESTS_TOPIC, ID_STATS_LEADERBOARD_REQUEST_TOPIC, ID_STATS_LEADERBOARD_RESPONSE_TOPIC};

use w3g_common::errors::Result;

use std::collections::HashMap;
use std::thread;
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
                Err(_) => IdStats::default(player.clone()),
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
                builder_stats.push(find_player_stats(&builder, &collection));
            }
            for titan in result.titans
            {
                titan_stats.push(find_player_stats(&titan, &collection));
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
                }
            }
        }
    }
}


fn convert_map_to_lobby(players: &HashMap<u8, Player>, collection: &Collection) -> Result<island_defense::lobby::Lobby>
{
    let mut builder_stats = HashMap::with_capacity(players.len());
    let mut builder_bbts = HashMap::with_capacity(players.len());
    let mut titan_stats = HashMap::with_capacity(1);
    let mut titan_bbts = HashMap::with_capacity(1);

    for slot in 0..10
    {
        if let Some(player) = players.get(&slot)
        {
            let stat = find_player_stats(player, collection);
            builder_bbts.insert(slot, stat.builder_stats.rating.clone()); 
            builder_stats.insert(slot, stat);
        }
    }

    for slot in 10..11
    {
        if let Some(player) = players.get(&slot)
        {
            let stat = find_player_stats(player, collection); 
            titan_bbts.insert(slot, stat.titan_stats.rating.clone());
            titan_stats.insert(slot, stat);
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

                builders.push(island_defense::lobby::Builder::new(slot, stat.player.name, stat.player.realm, rating, stat.builder_stats.wins, stat.builder_stats.losses, stat.builder_stats.ties));
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

                titans.push(island_defense::lobby::Titan::new(slot, stat.player.name, stat.player.realm, rating, stat.titan_stats.wins, stat.titan_stats.losses, stat.titan_stats.ties));
            },
            _ => bail!("Titan slot: {} did not have ratings computed", slot),
        };
    } 


    Ok(island_defense::lobby::Lobby::new(island_defense::lobby::BuilderTeam::new(builders, builders_rating), island_defense::lobby::TitanTeam::new(titans, titans_rating)))
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
            match convert_map_to_lobby(&lobby, &collection)
            {
                Err(error) => error!("Unable to convert lobby: {:?}, {}", lobby, error),
                Ok(lobby_stats) => 
                {
                    match producer.send_to_topic(ID_BULK_STATS_RESPONSES_TOPIC, key, lobby_stats)
                {
                    Err(error) => error!("Unable to send lobby stats: {}", error),
                    _ => (),
                }
                }
            } 
        }
    }
}

fn get_top_builders(size: i64, collection: &Collection) -> Result<Vec<island_defense::leaderboard::Builder>>
{
    let mut sort = Document::new();
    sort.insert("builder_stats.rating.mu", Bson::I32(-1));

    let mut options = FindOptions::new();
    options.limit = Some(size);
    options.sort = Some(sort);

    let builders: Vec<island_defense::leaderboard::Builder> = collection.find(None, Some(options))?
        .drain_current_batch()?
        .into_iter()
        .filter_map(|document| bson::from_bson(Bson::Document(document)).ok())
        .take(size as usize)
        .map(|stats: IdStats| island_defense::leaderboard::Builder::new(stats.player.name, stats.player.realm, stats.builder_stats.rating.mu(), stats.builder_stats.wins, stats.builder_stats.losses, stats.builder_stats.ties))
        .collect();

    Ok(builders)
}

fn get_top_titans(size: i64, collection: &Collection) -> Result<Vec<island_defense::leaderboard::Titan>>
{
    let mut sort = Document::new();
    sort.insert("titan_stats.rating.mu", Bson::I32(-1));

    let mut options = FindOptions::new();
    options.limit = Some(size);
    options.sort = Some(sort);

    let titans: Vec<island_defense::leaderboard::Titan> = collection.find(None, Some(options))?
        .drain_current_batch()?
        .into_iter()
        .filter_map(|document| bson::from_bson(Bson::Document(document)).ok())
        .take(size as usize)
        .map(|stats: IdStats| island_defense::leaderboard::Titan::new(stats.player.name, stats.player.realm, stats.titan_stats.rating.mu(), stats.titan_stats.wins, stats.titan_stats.losses, stats.titan_stats.ties))
        .collect();

    Ok(titans)
}

fn get_leaderboard(size: i64, collection: &Collection) -> Result<island_defense::leaderboard::LeaderBoard>
{
    if size < 0
    {
        bail!("Size: {} must be positive", size);
    }

    // TODO: Size should really be i32 so it can be set as the batch size for Mongo queries

    Ok(island_defense::leaderboard::LeaderBoard::new(get_top_builders(size, collection)?, get_top_titans(size, collection)?))
}

fn leaderboard_handler(mut consumer: PubSubConsumer, mut producer: PubSubProducer, collection: Collection)
{
    loop 
    {
        let requests: Vec<(u64, i64)> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Unable to parse leaderboard request: {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if requests.is_empty()
        {
            thread::yield_now();
        }

        for (key, size) in requests
        {
            match get_leaderboard(size, &collection)
            {
                Err(error) => error!("Unable to get leaderboard of size {}. {}", size, error),
                Ok(leaderboard) => 
                {
                    match producer.send_to_topic(ID_STATS_LEADERBOARD_RESPONSE_TOPIC, key, &leaderboard)
                {
                    Err(error) => error!("Unable to send leaderboard: {}", error),
                    _ => trace!("Sent key: {}, size: {}, leadboard: {:?}", key, size, leaderboard),
                }
                }
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
    let collection = database.collection(&mongo_collecion);
   
    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_BULK_STATS_REQUESTS_TOPIC, KAFKA_GROUP)
        .unwrap();

    let bulk_requests_thread = thread::spawn(move || {
        stats_bulk_request_handler(consumer, producer, collection);
    });

    // Stats Updates
    let collection = database.collection(&mongo_collecion);

    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_STATS_UPDATES_TOPIC, KAFKA_GROUP)
        .unwrap();

    let updates_thread = thread::spawn(move || {
        stats_update_handler(consumer, collection);
    });

    // Leaderboard
    let collection = database.collection(&mongo_collecion);

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_STATS_LEADERBOARD_REQUEST_TOPIC, KAFKA_GROUP)
        .unwrap();

    let leaderboard_thread = thread::spawn(move || {
        leaderboard_handler(consumer, producer, collection);
    });

    let _ = bulk_requests_thread.join(); 
    let _ = updates_thread.join();
    let _ = leaderboard_thread.join();
    
}
