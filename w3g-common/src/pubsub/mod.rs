pub mod producer;
use self::producer::PubSubProducer;

pub mod consumer;
use self::consumer::PubSubConsumer;

pub mod model;
use self::model::Message;

use super::errors::Result;

use std::{thread, time};
use std::collections::VecDeque;

/// Topic to Request the players in a lobby for a given bot 
pub const ID_LOBBY_REQUEST_TOPIC: &'static str = "id-lobby-request";

/// Topic to Request the stats of given players
pub const ID_STATS_REQUEST_TOPIC: &'static str = "id-stats-request";

/// Topic to Response the stats of given players
pub const ID_LOBBY_STATS_RESPONSE_TOPIC: &'static str = "id-lobby-stats-response";

/// Topic to Request a leaderboard
pub const ID_LEADER_BOARD_REQUEST_TOPIC: &'static str = "id-leaderboard-request";

/// Topic to Response to a leaderboard request
pub const ID_LEADER_BOARD_RESPONSE_TOPIC: &'static str = "id-leaderboard-response";

/// Topic to dump Replays
pub const ID_REPLAY_TOPIC: &'static str = "id-replay-response";

/// Topic to dump the results of a game (Winners / Losers)
pub const ID_GAME_RESULT_TOPIC: &'static str = "id-result-response";

/// Topic to Request/Response to verify Kafka is running
pub const W3G_LOOPBACK_TOPIC: &'static str = "w3g-router-loopback";


pub fn delay_until_kafka_ready<S>(broker_uris: &[String], group: S)
    where S: Into<String>
{
    let cloneable_group = group.into();
    loop {
        match perform_loopback_test(broker_uris, cloneable_group.to_owned())
        {
            Ok(_) => return,
            Err(error) => 
            {
                error!("Unable to loopback on kafka, delaying. {}", error);
                thread::sleep(time::Duration::from_secs(5));
            },
        }
    }
}

pub fn perform_loopback_test<S>(broker_uris: &[String], group: S) -> Result<()>
    where S: Into<String>
{
    let mut producer = PubSubProducer::new(broker_uris.to_owned().to_vec())?;
    let mut consumer = PubSubConsumer::new(broker_uris.to_owned().to_vec(), W3G_LOOPBACK_TOPIC, group)?;

    let message: Message<String> = Message::new(String::from("Hello, World!"), VecDeque::new(), None);

    producer.send_to_topic(W3G_LOOPBACK_TOPIC, 0, &message)?;
    let _: Vec<(u64, Message<String>)> = consumer.listen()?;

    Ok(())
}