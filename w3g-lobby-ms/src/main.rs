extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};
  
extern crate w3g_lobby_ms;

use w3g_lobby_ms::engine;

extern crate w3g_common;


use w3g_common::errors::Result;

use w3g_common::pubsub::producer::PubSubProducer;
use w3g_common::pubsub::consumer::PubSubConsumer;
use w3g_common::pubsub::ID_LOBBY_REQUEST_TOPIC;  

use std::thread::JoinHandle;
use std::{env, thread}; 
 
const KAFKA_GROUP: &'static str = "w3g-lobby-ms";
 
fn configure_logger() -> Result<()>
{
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    if let Ok(rust_log) = env::var("RUST_LOG")
    {
        builder.parse(&rust_log);
    } 
    builder.init();

    Ok(())
}

fn create_lobby_handler() -> Result<JoinHandle<()>>
{
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::delay_until_kafka_ready(&broker_uris, KAFKA_GROUP);

    let mut consumer = PubSubConsumer::new(broker_uris.clone(), ID_LOBBY_REQUEST_TOPIC, KAFKA_GROUP)?;
    let mut producer = PubSubProducer::new(broker_uris.clone())?;

    Ok(thread::spawn(move || {
        loop {
            match engine::lobby::run_service(&mut consumer, &mut producer)
            {
                Ok(_) => {},
                Err(error) => error!("{}", error),
            }
        }
    }))
}
 
 
fn main() {
    let _ = configure_logger()
                .unwrap();

    let lobby_handler = create_lobby_handler()
                        .unwrap();

    lobby_handler.join()
        .unwrap();
}
