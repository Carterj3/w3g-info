#![feature(plugin, decl_macro, custom_derive, integer_atomics)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate rocket; 
extern crate rocket_contrib;

use rocket::State;

use rocket_contrib::Json; 

extern crate chashmap;

use chashmap::CHashMap;

extern crate w3g_common;

use w3g_common::errors::Result; 

use w3g_common::pubsub::PubSubProducer;
use w3g_common::pubsub::PubSubConsumer;
use w3g_common::pubsub::model::IdStats; 
use w3g_common::pubsub::ID_BULK_STATS_RESPONSES_TOPIC;
use w3g_common::pubsub::ID_LOBBY_REQUESTS_TOPIC;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex}; 
use std::thread;
use std::collections::HashMap;

const KAFKA_GROUP: &'static str = "w3g-router-ms";

struct RustRouterConfig 
{
    pubsub_key: Arc<AtomicU64>,
    pubsub_producer: Arc<Mutex<PubSubProducer>>,
    stats_map: Arc<CHashMap<u64, HashMap<u8, IdStats>>>,
}

#[get("/")]
fn index() -> Result<Json<Vec<(u64, HashMap<u8, IdStats>)>>> {

    let broker_uris = vec!(String::from("kafka:9092"));  

    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let mut consumer = PubSubConsumer::new(broker_uris.clone(), ID_BULK_STATS_RESPONSES_TOPIC, KAFKA_GROUP)
        .unwrap();

    producer.send_to_topic(ID_LOBBY_REQUESTS_TOPIC, 1337, "")?;
    let responses: Vec<(u64, HashMap<u8, IdStats>)> = consumer.listen()?;

    for (key, players) in responses.clone()
    {
        trace!("Received stats response for key: {:?}={:?}", key, players);
    }
    
    Ok(Json(responses))
}


///
/// Gets the stats of all the players in the current lobby for the given MapType
///
/// * `map` - the name of the map (i.e. Island Defense)
/// * `common` - the stored common configuration needed to do anything (i.e kafka parameters)
#[get("/lobby/<map>")]
fn lobby( map: String
        , common: State<RustRouterConfig>) -> Result<Json<HashMap<u8, IdStats>>>
{
    if map.is_empty()
    {
        bail!("<map> not specified");
    }

    let mut producer = match common.pubsub_producer.lock()
    {
        Ok(producer) => producer,
        Err(_) => bail!("Poisioned"),
    };
    let key = common.pubsub_key.fetch_add(1, Ordering::SeqCst);

    producer.send_to_topic("id-lobby-requests", key, "")?;

    trace!("Waiting for lobby response for key: {:?}", key);
    while !common.stats_map.contains_key(&key)
    {
        thread::yield_now();
    }

    let lobby = common.stats_map.remove(&key)
        .ok_or("Lobby data was empty")?;

    Ok(Json(lobby))
}

fn bulk_stats_handler(mut consumer: PubSubConsumer, stats_map: Arc<CHashMap<u64, HashMap<u8, IdStats>>>)
{
    loop {
        let bulk_stats: Vec<(u64, HashMap<u8, IdStats>)> = match consumer.listen()
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
        }

        for (key, bulk_stat) in bulk_stats
        { 
            trace!("Received lobby response for key: {:?} = {:?}", key, bulk_stat);

            stats_map.insert(key, bulk_stat);
        }
    }
}

fn main() {
    let broker_uris = vec!(String::from("kafka:9092")); 

    let producer = PubSubProducer::new(broker_uris.clone())
             .unwrap();
    let producer = Arc::new(Mutex::new(producer));

    let key = Arc::new(AtomicU64::new(0));
    let map = Arc::new(CHashMap::new());

    let router_config = RustRouterConfig {
        pubsub_key: key,
        pubsub_producer: producer,
        stats_map: Arc::clone(&map),
    };

    let topic = String::from("id-bulk-stats-responses");
    let group = String::from("w3g-router-ms");

    let consumer = PubSubConsumer::new(broker_uris.clone(), topic, group)
        .unwrap();

     let bulk_responses_thread = thread::spawn(move || {
        bulk_stats_handler(consumer, Arc::clone(&map));
    });

    rocket::ignite()
        .mount("v1", routes![index, lobby])
        .manage(router_config)
        .launch();

    let _ = bulk_responses_thread.join();
}
