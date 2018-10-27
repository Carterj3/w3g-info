#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Builder
{
    pub slot: u8,
    pub name: String,
    pub realm: String,
    pub rating: Rating,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Titan
{
    pub slot: u8,
    pub name: String,
    pub realm: String,
    pub rating: Rating,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Rating
{
    pub mean_rating: f64,
    pub potential_gain: f64,
    pub potential_loss: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct BuilderTeam
{
    pub players: Vec<Builder>,
    pub team_rating: Rating,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct TitanTeam
{
    pub players: Vec<Titan>,
    pub team_rating: Rating,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct Lobby
{
    pub builders: BuilderTeam,
    pub titans: TitanTeam,
}