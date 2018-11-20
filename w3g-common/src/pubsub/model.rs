use bbt::Rating;
use std::hash::{Hash, Hasher};

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T>
 {
    pub data: T,
    pub destinations: VecDeque<String>,
    pub debug: Option<HashMap<String, String>>,
}

impl <T> Message<T>
{
    pub fn new(data: T, destinations: VecDeque<String>, debug: Option<HashMap<String, String>>) -> Message<T> 
    {
        Message {
            data,
            destinations,
            debug,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub name: String,
    pub realm: String,
}

impl Player
{
    pub fn new<S1, S2>(name: S1, realm: S2) -> Player
        where S1: Into<String>, S2: Into<String>
    {
        Player {
            name: name.into(),
            realm: realm.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum IdTeam
{
    Builder,
    Titan,
    Tie,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct IdGameResult {
    pub builders: Vec<Player>,
    pub titans: Vec<Player>,
    pub winner: IdTeam,
}

impl IdGameResult
{
    pub fn new(builders: Vec<Player>, titans: Vec<Player>, winner: IdTeam) -> IdGameResult
    {
        IdGameResult {
            builders,
            titans,
            winner,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash)]
pub struct IdStats {
    pub player: Player,
    pub builder_stats: BuilderStats,
    pub titan_stats: TitanStats,
}

impl IdStats
{
    pub fn default(player: Player) -> IdStats
    {
        IdStats {
            player,
            builder_stats: BuilderStats::default(),
            titan_stats: TitanStats::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BuilderStats
{
    pub rating: Rating,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl BuilderStats
{
    pub fn default() -> BuilderStats
    {
        BuilderStats {
            rating: Rating::new(1500.0, 1500.0 / 3.0),
            wins: 0,
            losses: 0,
            ties: 0,
        }
    }
}

impl Hash for BuilderStats {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /* f64 (rating) cannot be hashed */
        self.wins.hash(state);
        self.losses.hash(state);
        self.ties.hash(state);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TitanStats
{
    pub rating: Rating,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl TitanStats
{
    pub fn default() -> TitanStats
    {
        TitanStats {
            rating: Rating::new(1500.0, 1500.0 / 3.0),
            wins: 0,
            losses: 0,
            ties: 0,
        }
    }
}

impl Hash for TitanStats {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /* f64 (rating) cannot be hashed */
        self.wins.hash(state);
        self.losses.hash(state);
        self.ties.hash(state);
    }
}