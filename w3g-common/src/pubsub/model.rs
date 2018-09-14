
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Player {
    pub name: String,
    pub realm: String,
}

impl Player
{
    pub fn new(name: String, realm: String) -> Player
    {
        Player {
            name,
            realm,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IdTeam
{
    Builder,
    Titan,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IdGameResult {
    pub builders: Vec<Player>,
    pub titans: Vec<Player>,
    pub winner: IdTeam,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    pub rating: f32,
    pub wins: u32,
    pub losses:u32,
}

impl BuilderStats
{
    pub fn default() -> BuilderStats
    {
        BuilderStats {
            rating: 1500.0,
            wins: 0,
            losses: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TitanStats
{
    pub rating: f32,
    pub wins: u32,
    pub losses:u32,
}

impl TitanStats
{
    pub fn default() -> TitanStats
    {
        TitanStats {
            rating: 1500.0,
            wins: 0,
            losses: 0,
        }
    }
}