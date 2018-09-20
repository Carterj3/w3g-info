#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

use env_logger::{Builder, Target};

extern crate w3g_common;

use w3g_common::parser::{Replay, ReplayBlock, Action, Command};
use w3g_common::pubsub::PubSubProducer;
use w3g_common::pubsub::PubSubConsumer;
use w3g_common::pubsub::IdGameResult;
use w3g_common::pubsub::IdTeam;
use w3g_common::pubsub::Player;
use w3g_common::pubsub::ID_REPLAY_RESPONSES_TOPIC;
use w3g_common::pubsub::ID_STATS_UPDATES_TOPIC;

use w3g_common::errors::Result;

use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use std::env;
use std::thread;

const KAFKA_GROUP: &'static str = "id-rating-ms";

fn get_game_result(player_list: &Vec<Player>, replay: &Replay) -> Result<IdGameResult>
{
    /*
        So players is real annoying-ish. From the website they're listed in order but I don't particularly want to just assign then 0-11.
        Also, ID.Dat is 0-indexed while replays appear to be 1-index'd
    */
    let mut player_indicies = Vec::with_capacity(player_list.len());
    player_indicies.push(replay.game_header.replay_saver.player_id - 1);
    for player in replay.game_header.players.iter()
    {
        player_indicies.push(player.player_id - 1);
    }
    player_indicies.sort();

    let players: HashMap<u8, &Player> = player_indicies.into_iter().zip(player_list.iter()).collect();

    let mut builders: HashSet<Player> = HashSet::new();
    let mut titans: HashSet<Player> = HashSet::new();
    let mut result = IdTeam::Tie;

    let mut game_started = false;

    for block in replay.replay_blocks.iter()
    {
        match block
        {
            ReplayBlock::Desync {tick_count, checksum, remaining_players} => warn!("desync occured: count: {:?}, checksum?: {:?}, remaing: {:?}", tick_count, checksum, remaining_players),
            ReplayBlock::Tick { num_bytes: _, time_increment: _, commands} =>
            {
                let tuple = extract_players_from_commands(commands, &players, game_started, result, builders, titans);
                game_started = tuple.0;
                result = tuple.1;
                builders = tuple.2;
                titans = tuple.3;
            },
            ReplayBlock::TickPreOverflow { num_bytes: _, time_increment: _, commands} =>
            {
                let tuple = extract_players_from_commands(commands, &players, game_started, result, builders, titans);
                game_started = tuple.0;
                result = tuple.1;
                builders = tuple.2;
                titans = tuple.3;
            },
            _ => {},
        }
    }

    if builders.is_empty() || titans.is_empty()
    {
        bail!("Builders: {} or Titans: {} were empty.", builders.len(), titans.len());
    }

    Ok(IdGameResult::new(builders.into_iter().collect(), titans.into_iter().collect(), result))
}


fn extract_players_from_commands(commands: &Vec<Command>, players: &HashMap<u8, &Player>, mut game_started: bool, mut result: IdTeam, mut builders: HashSet<Player>, mut titans: HashSet<Player>) -> (bool, IdTeam, HashSet<Player>, HashSet<Player>)
{
    for command in commands.iter()
    {
        for action in command.actions.iter()
        {
            match action
            {
                Action::SyncStoredInteger {file, group, key, value} =>
                {
                    trace!("f: {:?}, g: {:?}, k: {:?}, v: {:?}", file, group, key, value);

                    match (file.as_str(), group.as_str(), key.parse::<u8>())
                    {
                        ("ID.D", "flag", Ok(player_index)) => 
                        {
                            match(players.get(&player_index), value)
                            {
                                (Some(player), 0) =>
                                {
                                    debug!("Player: {:?}, lost", player);
                                    if builders.contains(player)
                                    {
                                        result = IdTeam::Titan;

                                    }else if titans.contains(player)
                                    {
                                        result = IdTeam::Builder;
                                    }
                                },
                                (Some(player), 1) =>
                                {
                                    debug!("Player: {:?}, won", player);
                                    if builders.contains(player)
                                    {
                                        result = IdTeam::Builder;

                                    }else if titans.contains(player)
                                    {
                                        result = IdTeam::Titan;
                                    }
                                },
                                ( debug_name, debug_value) =>
                                {
                                    error!("Name[{:?}]: {:?}, Value: {:?} are not desired for `flag`", player_index, debug_name, debug_value);
                                }
                            }
                        },
                        ("ID.D", "class", Ok(player_index)) =>
                        {
                            if game_started
                            {
                                continue;
                            }

                            match (players.get(&player_index), value)
                            {
                                /* 
                                    public static constant integer CLASS_NONE = 0;
                                    public static constant integer CLASS_MINION = 1;
                                    public static constant integer CLASS_TITAN = 2;
                                    public static constant integer CLASS_DEFENDER = 3;
                                    public static constant integer CLASS_OBSERVER = 4;
                                */
                                (Some(player), 4) =>
                                {
                                    debug!("Player: {:?} is an observer (builder)", player);
                                    builders.insert((*player).clone());
                                }
                                (Some(player), 3) =>
                                {
                                    debug!("Player: {:?} is a builder", player);
                                    builders.insert((*player).clone());
                                },
                                (Some(player), 2) =>
                                {
                                    debug!("Player: {:?} is a titan", player);
                                    titans.insert((*player).clone());
                                },
                                (Some(player), 1) =>
                                {
                                    debug!("Player: {:?} is a minion (builder)", player);
                                    builders.insert((*player).clone());
                                },
                                (Some(player), 0) =>
                                {
                                    debug!("Player: {:?} is a <none> (builder)", player);
                                    builders.insert((*player).clone());
                                },
                                ( debug_player, debug_value) =>
                                {
                                    error!("Player[{:?}]: {:?}, Value: {:?} are not desired for `class`", player_index, debug_player, debug_value);
                                }
                            } 
                        },
                        ("ID.D", "game_start", _) =>
                        {
                            game_started = true;
                        }
                        _ => {}
                    }
                    
                },
                _ => {},
            }
        }
    }

    (game_started, result, builders, titans)
}

fn replays_handler(mut consumer: PubSubConsumer, mut producer: PubSubProducer)
{
    let one_hour = Duration::from_secs(60*60);
    

    loop
    {
        let games: Vec<(u64, (Vec<Player>, Replay))> = match consumer.listen()
        {
            Err(error) =>
            {
                error!("Failed to get pubsub data: {}", error);
                continue;
            },
            Ok(data) => data,
        };

        if games.is_empty()
        {
            thread::yield_now();
        }

        for (game_id, (players, replay)) in games
        {
            let mut result = match get_game_result(&players, &replay)
            {
                Err(error) =>
                {
                    error!("Fail to get result from game: {} because {}", game_id, error);
                    continue;
                },
                Ok(data) => data,
            };

            let duration = Duration::from_millis(replay.replay_header.duration as u64);
            if duration > one_hour && result.winner == IdTeam::Titan
            {
                result.winner = IdTeam::Tie;
            }

            match producer.send_to_topic(ID_STATS_UPDATES_TOPIC, game_id, &result)
            {
                Err(error) => error!("Failed to send result for game_id: {} because {}", game_id, error),
                _ => info!("Sent stats for game_id: {}. Result: {:?}", game_id, result),
            }

        }
    }
}

fn main() {
    /* Log */
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init();

    /* Kafka */
    let broker_uris = match env::var("KAFKA_URIS")
    {
        Ok(uris) => vec!(uris),
        Err(_) => vec!(String::from("localhost:9092")),
    };
    w3g_common::pubsub::perform_loopback_test(&broker_uris, KAFKA_GROUP)
        .expect("Kafka not initialized yet");

    let producer = PubSubProducer::new(broker_uris.clone())
        .unwrap();
    let consumer = PubSubConsumer::new(broker_uris.clone(), ID_REPLAY_RESPONSES_TOPIC, KAFKA_GROUP)
        .unwrap();

    let replay_thread = thread::spawn(move || {
        replays_handler(consumer, producer);
    });

    let _ = replay_thread.join();

}
