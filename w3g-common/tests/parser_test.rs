extern crate w3g_common; 

extern crate serde;
extern crate serde_json;
extern crate rmp_serde;
extern crate bincode;

use w3g_common::parser::Replay;

use serde::{Deserialize, Serialize};

#[test]
fn test_smoke_11151616() {
    w3g_common::parser::extract_replay("resources/11151616.w3g").unwrap();
}

#[test]
fn test_smoke_11151801() {
    w3g_common::parser::extract_replay("resources/11151801.w3g").unwrap();
}

#[test]
fn test_smoke_11151811() {
    w3g_common::parser::extract_replay("resources/11151811.w3g").unwrap();
}

#[test]
fn test_replay_to_json()
{
    // bytes 2,810,760
    let original_replay = w3g_common::parser::extract_replay("resources/11151811.w3g").unwrap();

    let serialized = serde_json::to_string(&original_replay).unwrap();
    let deserialized: Replay = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original_replay, deserialized);
    println!("(json) replay size: {}", serialized.len());
}


#[test]
fn test_replay_to_rmps()
{
    // 441,720 bytes
    let original_replay = w3g_common::parser::extract_replay("resources/11151811.w3g").unwrap();
    
    let mut serialized = Vec::new();
    original_replay.serialize(&mut rmp_serde::Serializer::new(&mut serialized)).unwrap();
    
    let mut de = rmp_serde::Deserializer::new(&serialized[..]);
    let deserialized: Replay = Deserialize::deserialize(&mut de).unwrap();

    assert_eq!(original_replay, deserialized);
    println!("(rmp) replay size: {}", serialized.len());
}

#[test]
fn test_replay_to_bincode()
{
    // 590,823 bytes
    let original_replay = w3g_common::parser::extract_replay("resources/11151811.w3g").unwrap();

    let serialized : Vec<u8> = bincode::serialize(&original_replay).unwrap();
    let deserialized: Replay = bincode::deserialize(&serialized).unwrap();

    assert_eq!(original_replay, deserialized);
    println!("(bincode) replay size: {}", serialized.len());
}
 