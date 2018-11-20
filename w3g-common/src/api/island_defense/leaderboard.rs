use ::pubsub::model::IdStats;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Builder
{
    pub name: String,
    pub realm: String,
    pub rating: f64,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl Builder {
    pub fn from_id_stats(stats: IdStats) -> Builder
    {
        Builder {
            name: stats.player.name,
            realm: stats.player.realm,
            rating: stats.builder_stats.rating.mu(),
            wins: stats.builder_stats.wins,
            losses: stats.builder_stats.losses,
            ties: stats.builder_stats.ties,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Titan
{
    pub name: String,
    pub realm: String,
    pub rating: f64,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

impl Titan {
    pub fn from_id_stats(stats: IdStats) -> Titan
    {
        Titan {
            name: stats.player.name,
            realm: stats.player.realm,
            rating: stats.titan_stats.rating.mu(),
            wins: stats.titan_stats.wins,
            losses: stats.titan_stats.losses,
            ties: stats.titan_stats.ties,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct LeaderBoard
{
    pub builders: Vec<Builder>,
    pub titans: Vec<Titan>,
}

