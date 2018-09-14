#[macro_use]
extern crate log;
extern crate env_logger;

extern crate mongodb;
#[macro_use] extern crate bson;

use mongodb::coll::Collection;
use mongodb::ThreadedClient;
use mongodb::Client; 
use mongodb::db::ThreadedDatabase; 

use bson::Bson::Document;

extern crate w3g_common; 
 
use w3g_common::pubsub::PubSubConsumer;

use w3g_common::pubsub::PubSubProducer;

use w3g_common::pubsub::model::Player;
use w3g_common::pubsub::model::IdStats; 
use w3g_common::pubsub::model::IdTeam;
use w3g_common::pubsub::model::IdGameResult;

use w3g_common::pubsub::ID_BULK_STATS_RESPONSES_TOPIC;

use std::collections::HashMap;
use std::thread;

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

fn stats_update_handler(mut consumer: PubSubConsumer, collection: Collection)
{
    loop {
        let games: Vec<(u64, IdGameResult)> = match consumer.listen()
        {
            Err(_) =>
            {
                error!("Error?");
                continue;
            },
            Ok(data) => data,
        };

        if games.is_empty()
        {
            thread::yield_now();
        }

        for (_, game) in games
        {
            let (titan_wins, titan_losses, builder_wins, builder_losses) = match game.winner
            {
                IdTeam::Builder => (0, 1, 1, 0),
                IdTeam::Titan => (1,0, 0, 1),
            };

            for builder in game.builders
            {
                let mut stats = find_player_stats(builder, &collection);
                stats.builder_stats.wins += builder_wins;
                stats.builder_stats.losses += builder_losses;

                match bson::to_bson(&stats)
                {
                    Ok(Document(document)) => {
                        let filter = doc!{
                            "player.name" => stats.player.name.clone(),
                            "player.realm" => stats.player.realm.clone(),
                        };

                        match collection.update_one(filter, document, None)
                        {
                            Err(_) => error!("Error, unable to save updates to player"),
                            Ok(_) => (),
                        }
                    },
                    Ok(_) => error!("Error, BSON was not converted to Document"),
                    Err(_) => error!("Error, unable to convert the BSON object"),
                }
            }

            for titan in game.titans
            {
                let mut stats = find_player_stats(titan, &collection);
                stats.titan_stats.wins += titan_wins;
                stats.titan_stats.losses += titan_losses;

                match bson::to_bson(&stats)
                {
                    Ok(Document(document)) => {
                        let filter = doc!{
                            "player.name" => stats.player.name.clone(),
                            "player.realm" => stats.player.realm.clone(),
                        };

                        match collection.update_one(filter, document, None)
                        {
                            Err(_) => error!("Error, unable to save updates to player"),
                            Ok(_) => (),
                        }
                    },
                    Ok(_) => error!("Error, BSON was not converted to Document"),
                    Err(_) => error!("Error, unable to convert the BSON object"),
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

fn stats_request_handler(mut consumer: PubSubConsumer, mut producer: PubSubProducer, collection: Collection)
{
    loop 
    {
        let players: Vec<(u64, Player)> = match consumer.listen()
        {
            Err(_) =>
            {
                error!("Error?");
                continue;
            },
            Ok(data) => data,
        };

        if players.is_empty()
        {
            thread::yield_now();
        }

        for (key, player) in players
        {
            let player = find_player_stats(player, &collection);

            match producer.send_to_topic("id-stats-responses", key, player)
            {
                Err(_) => error!("Error?"),
                _ => (),
            }
        }
    }
}

fn main() {
    env_logger::init();

    let client = Client::connect("mongodb", 27017)
        .unwrap();
    let database = client.db("island-defense");

    // TODO: Create index on player-stats for `player.name`+`player.realm`

    // Stats Requests
    let collection = database.collection("player-stats");

    let broker_uris = vec!(String::from("kafka:9092"));
    let topic = String::from("id-stats-requests");
    let group = String::from("id-stats-ms");

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), topic, group)
        .unwrap();

    let requests_thread = thread::spawn(move || {
        stats_request_handler(consumer, producer, collection);
    });

    // Bulk Stats Requests
    let collection = database.collection("player-stats");

    let broker_uris = vec!(String::from("kafka:9092"));
    let topic = String::from("id-bulk-stats-requests");
    let group = String::from("id-stats-ms");

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), topic, group)
        .unwrap();

    let bulk_requests_thread = thread::spawn(move || {
        stats_bulk_request_handler(consumer, producer, collection);
    });

    // Stats Updates
    let collection = database.collection("player-stats");

    let topic = String::from("id-stats-updates");
    let group = String::from("id-stats-ms");

    let consumer = PubSubConsumer::new(broker_uris.clone(), topic, group)
        .unwrap();

    let updates_thread = thread::spawn(move || {
        stats_update_handler(consumer, collection);
    });

    let _ = bulk_requests_thread.join();
    let _ = requests_thread.join();
    let _ = updates_thread.join();
    
}
