extern crate w3g_common;

use w3g_common::pubsub::model::{Player, IdTeam};

use std::collections::HashMap;


fn players_in_11151616() -> (IdTeam, HashMap<u8, Player>)
{
    let mut players = HashMap::new();

    players.insert(0, Player::new("grumble007", "useast.battle.net"));
    players.insert(1, Player::new("nixon", "useast.battle.net"));
    players.insert(2, Player::new("itsjustaprnkbro", "uswest.battle.net"));
    players.insert(3, Player::new("kimimaru", "uswest.battle.net"));
    players.insert(4, Player::new("thewqlf", "uswest.battle.net"));
    players.insert(5, Player::new("ggez", "uswest.battle.net"));
    players.insert(6, Player::new("kitten411", "uswest.battle.net"));
    players.insert(7, Player::new("hashcakes", "useast.battle.net"));
    players.insert(8, Player::new("taling", "useast.battle.net"));
    players.insert(9, Player::new("bongrip", "useast.battle.net"));
    players.insert(10, Player::new("ougi", "useast.battle.net"));

    (IdTeam::Builder, players)
}

#[test]
fn test_11151616() {


    w3g_common::parser::extract_replay("resources/11151616.w3g").unwrap();
}