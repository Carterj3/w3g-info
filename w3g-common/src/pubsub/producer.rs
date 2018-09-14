
use std::time::Duration;
use std::fmt::Debug;

use kafka::producer::{Producer, Record, RequiredAcks};

use rmp_serde::Serializer;

use serde::Serialize;

use byteorder::{WriteBytesExt, BigEndian};

use ::errors::*;

pub struct PubSubProducer 
{
    producer: Producer,
}

impl PubSubProducer
{
    pub fn new(broker_uris: Vec<String>) -> Result<PubSubProducer>
    {
        let producer = Producer::from_hosts(broker_uris)
             // ~ give the brokers one second time to ack the message
             .with_ack_timeout(Duration::from_secs(1))
             // ~ require only one broker to ack the message
             .with_required_acks(RequiredAcks::One)
             // ~ build the producer with the above settings
             .create()?;

        Ok(PubSubProducer
        {
            producer,
        })
    }

    /*
        let mut serialized = Vec::new();
        original_replay.serialize(&mut rmp_serde::Serializer::new(&mut serialized)).unwrap();
        
        let mut de = rmp_serde::Deserializer::new(&serialized[..]);
        let deserialized: Replay = Deserialize::deserialize(&mut de).unwrap();
    */

    pub fn send_to_topic<D>(&mut self, topic: &str, key: u64, value: D) -> Result<()>
        where D: Serialize+Debug
    {
        debug!("Sending? topic: {:?}, key: {:?}, value: {:?}", topic, key, value);

        let mut key_bytes = Vec::new();
        key_bytes.write_u64::<BigEndian>(key)?;

        let mut serialized = Vec::new();
        value.serialize(&mut Serializer::new(&mut serialized))?;

        /* TODO: Compress data */

        self.producer.send(&Record {
            topic: topic,
            partition: -1,
            key: key_bytes,
            value: serialized,
        })?;

        Ok(())
    }
}