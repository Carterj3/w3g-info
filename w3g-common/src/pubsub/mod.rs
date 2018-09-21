pub mod producer;

pub use self::producer::PubSubProducer;

pub mod consumer;

pub use self::consumer::PubSubConsumer;

pub mod model;

pub use self::model::Player;
pub use self::model::IdStats;
pub use self::model::BuilderStats;
pub use self::model::TitanStats;

pub use self::model::IdTeam;
pub use self::model::IdGameResult;

use super::errors::Result;


pub const W3G_LOOPBACK_TOPIC: &'static str = "w3g-router-loopback";

pub const ID_BULK_STATS_REQUESTS_TOPIC: &'static str = "id-bulk-stats-requests";
pub const ID_BULK_STATS_RESPONSES_TOPIC: &'static str = "id-bulk-stats-responses";
pub const ID_STATS_UPDATES_TOPIC: &'static str = "id-stats-updates";

pub const ID_LOBBY_REQUESTS_TOPIC: &'static str = "id-lobby-requests";

pub const ID_REPLAY_RESPONSES_TOPIC: &'static str = "id-replays-responses";


pub fn perform_loopback_test<S>(broker_uris: &Vec<String>, group: S) -> Result<()>
    where S: Into<String>
{
    let mut producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let mut consumer = PubSubConsumer::new(broker_uris.clone(), W3G_LOOPBACK_TOPIC, group)
        .unwrap();

    producer.send_to_topic(W3G_LOOPBACK_TOPIC, 1337, "Hello World")?;
    let _: Vec<(u64, String)> = consumer.listen()?;

    Ok(())
}