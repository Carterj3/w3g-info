
use engine::ent;

use w3g_common::pubsub::consumer::PubSubConsumer;
use w3g_common::pubsub::producer::PubSubProducer;
use w3g_common::pubsub::model::{Message, Player};

use w3g_common::errors::Result;

use std::collections::HashMap;

pub fn run_service(consumer: &mut PubSubConsumer, producer: &mut PubSubProducer) -> Result<()>
{
    let messages: Vec<(u64, Message<u64>)> = consumer.listen()?;

    for (key, mut message) in messages.into_iter()
    { 
        match message.destinations.pop_front()
        {
            None => warn!("Received message without destinations for key: {}", key),
            Some(topic) =>
            {
                let bot_id = format!("{}", message.data);
                let destinations = message.destinations;
                let debug = message.debug;

                let players = ent::get_players_for_bot(&bot_id)?;
                let response: Message<HashMap<u32, Player>> = Message::new(players, destinations, debug);
                producer.send_to_topic(&topic, key, &response)?;

                trace!("Sent response for bot: {} to topic: {}", bot_id, topic);
            }
        }
    }

    Ok(())
}