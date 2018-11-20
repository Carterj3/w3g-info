
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};

use rmp_serde::Deserializer;

use serde::de::{Deserialize, DeserializeOwned};

use byteorder::{BigEndian, ReadBytesExt};

use std::io::Cursor;
use std::fmt::Debug;

use pubsub::model::Message;
use ::errors::*;

pub struct PubSubConsumer 
{
    consumer: Consumer,
}

impl PubSubConsumer  
{
    /// Topic is the "Queue/Topic" to listen on
    /// Group is the consumer who is listening. If a member of the group commits a message then it counts as read for all members.
    pub fn new<S1, S2>(broker_uris: Vec<String>, topic: S1, group: S2) -> Result<PubSubConsumer>
        where S1: Into<String>, S2: Into<String>
    {
        let consumer: Consumer = Consumer::from_hosts(broker_uris)
                .with_topic(topic.into())
                .with_group(group.into())
                .with_fallback_offset(FetchOffset::Earliest)
                .with_offset_storage(GroupOffsetStorage::Kafka)
                .with_fetch_max_bytes_per_partition(20000000)
                .with_retry_max_bytes_limit(20000000)
                .create()?;

        Ok(
            PubSubConsumer {
                consumer,
            }
        )
    }

    pub fn listen<D>(&mut self) -> Result<Vec<(u64, Message<D>)>>
        where D: DeserializeOwned+Debug
    {
        let mut data = Vec::new();

        let message_sets = self.consumer.poll()?;

        for message_set in message_sets.iter()
        {
            for message in message_set.messages()
            {
                let serialized = message.value;
                let key = message.key.to_owned();
 
                let key = match Cursor::new(key.clone()).read_u64::<BigEndian>()
                {
                    Err(_) =>
                    {
                        error!("Error? failed to deserialize key");
                        continue;
                    },
                    Ok(data) => data,
                };

                let mut deserializer = Deserializer::new(serialized);
                let value = match Deserialize::deserialize(&mut deserializer)
                {
                    Err(_) =>
                    {
                        error!("Error? failed to deserialize value");
                        continue;
                    },
                    Ok(data) => data,
                };

                trace!("Received key: {:?}, value: {:?}", key, value);

                data.push((key, value));
            }

            self.consumer.consume_messageset(message_set)?;
        }

        self.consumer.commit_consumed()?;

        Ok(data)
    }
}
