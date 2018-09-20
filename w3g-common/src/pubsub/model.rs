use bbt::Rating;


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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IdTeam
{
    Builder,
    Titan,
    Tie,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IdStats {
    pub player: Player,
    pub rating: Rating,
    pub builder_stats: BuilderStats,
    pub titan_stats: TitanStats,
}

impl IdStats
{
    pub fn default(player: Player) -> IdStats
    {
        IdStats {
            player,
            rating: Rating::new(1500.0, 1500.0 / 3.0),
            builder_stats: BuilderStats::default(),
            titan_stats: TitanStats::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BuilderStats
{
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl BuilderStats
{
    pub fn default() -> BuilderStats
    {
        BuilderStats {
            wins: 0,
            losses: 0,
            ties: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TitanStats
{
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl TitanStats
{
    pub fn default() -> TitanStats
    {
        TitanStats {
            wins: 0,
            losses: 0,
            ties: 0,
        }
    }
}