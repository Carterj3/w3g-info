extern crate bbt; 
use self::bbt::Rater;

extern crate num;   
use self::num::Integer;

use ::errors::Result;

use std::cmp::Eq;
use std::hash::Hash;
use std::collections::HashMap;

/// Computes what the new ratings could be for if either of the teams won
/// 
/// # Arguments
/// * `team1` The ratings for the players in team 1
/// * `team2` The ratings for the players in team 2
/// 
/// # Return
/// * A tuple where `.0` is what the ratings would become if team1 beat team2 and `.1` is if team2 beat team1
pub fn compute_potential_ratings<T>(team1: &HashMap<T, bbt::Rating>, team2: &HashMap<T, bbt::Rating>) -> Result<((HashMap<T, bbt::Rating>, HashMap<T, bbt::Rating>), (HashMap<T, bbt::Rating>, HashMap<T, bbt::Rating>))>
    where T: Clone+Eq+Hash
{
    let mut team1_keys = Vec::with_capacity(team1.len());
    let mut team1_ratings = Vec::with_capacity(team1.len());

    for (key, rating) in team1.iter()
    {
        team1_keys.push(key.clone());
        team1_ratings.push(rating.clone());
    }

    let mut team2_keys = Vec::with_capacity(team2.len());
    let mut team2_ratings = Vec::with_capacity(team2.len());

    for (key, rating) in team2.iter()
    {
        team2_keys.push(key.clone());
        team2_ratings.push(rating.clone());
    }

    let (team1_wins_team1, team1_wins_team2) = compute_new_ratings(&team1_ratings, &team2_ratings)?;
    let team1_wins_team1: HashMap<T, bbt::Rating> = team1_keys.clone().into_iter().zip(team1_wins_team1.into_iter()).collect();
    let team1_wins_team2: HashMap<T, bbt::Rating> = team2_keys.clone().into_iter().zip(team1_wins_team2.into_iter()).collect();
 
    let (team2_wins_team2, team2_wins_team1) = compute_new_ratings(&team2_ratings, &team1_ratings)?;
    let team2_wins_team1: HashMap<T, bbt::Rating> = team1_keys.clone().into_iter().zip(team2_wins_team1.into_iter()).collect();
    let team2_wins_team2: HashMap<T, bbt::Rating> = team2_keys.clone().into_iter().zip(team2_wins_team2.into_iter()).collect();

    Ok( ((team1_wins_team1, team1_wins_team2), (team2_wins_team1, team2_wins_team2)) )
}

/// Computes what the new ratings are 
/// 
/// # Arguments
/// * `winners` The ratings for the winning team
/// * `losers` The ratings for the losing team
/// 
/// # Return
/// * a tuple where `.0` is the new ratings for the winning team and `.1` is the new ratings for the losing team
pub fn compute_new_ratings(winners: &Vec<bbt::Rating>, losers: &Vec<bbt::Rating>) -> Result<(Vec<bbt::Rating>, Vec<bbt::Rating>)>
{
    if winners.is_empty() || losers.is_empty()
    {
        let winner_ratings = winners.clone();
        let loser_ratings = losers.clone();

        Ok( (winner_ratings, loser_ratings) )
    } else 
    {
        let rater = Rater::new(1500.0 / 6.0);
        let lcm = winners.len().lcm(&losers.len());

        let mut winner_ratings: Vec<bbt::Rating> = Vec::with_capacity(lcm);
        let mut loser_ratings: Vec<bbt::Rating> = Vec::with_capacity(lcm);

        /* Resulting ratings with uneven teams are pretty garbage so make the teams appear the same size */
        for i in 0..lcm
        {
            winner_ratings.push(winners.get(i % winners.len())
                .ok_or(format!("Bad index: {} of {}", i, winners.len()))?
                .clone());
            
            loser_ratings.push(losers.get(i % losers.len())
                .ok_or(format!("Bad index: {} of {}", i, losers.len()))?
                .clone());
        }

        /* bbt requires owned objects so can't get around cloning */
        let mut new_ratings = rater.update_ratings(vec!(winner_ratings, loser_ratings), vec!(1, 2))?.into_iter();

        let mut winner_ratings = new_ratings.next()
            .ok_or("None found for winner's new ratings")?;
        winner_ratings.truncate(winners.len());

        let mut loser_ratings = new_ratings.next()
            .ok_or("None found for loser's new ratings")?;
        loser_ratings.truncate(losers.len());
        
        Ok((winner_ratings, loser_ratings))
    } 
}