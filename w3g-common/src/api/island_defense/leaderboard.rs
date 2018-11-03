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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, new)]
pub struct LeaderBoard
{
    pub builders: Vec<Builder>,
    pub titans: Vec<Titan>,
}