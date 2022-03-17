use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::anyhow;
use colored::*;
use serde::{Deserialize, Serialize};

use crate::teams::Team;

/// Bracket regions
#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum Region {
    West,
    South,
    Midwest,
    East,
}

impl Region {
    /// Indexed such that the winner of Region 0 plays 1, 2 plays 3
    pub fn to_ind(self) -> usize {
        match self {
            Self::West => 0,
            Self::East => 1,
            Self::South => 2,
            Self::Midwest => 3,
        }
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::South => "South",
                Self::Midwest => "Midwest",
                Self::West => "West",
                Self::East => "East",
            }
        )
    }
}

impl FromStr for Region {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "south" => Ok(Self::South),
            "midwest" => Ok(Self::Midwest),
            "west" => Ok(Self::West),
            "east" => Ok(Self::East),
            _ => Err(anyhow!("Unexpected region {}", s)),
        }
    }
}

/// Seed in the bracket
#[derive(Copy, Clone, Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct Seed(pub u8);

impl Seed {
    pub fn new(seed: u8) -> anyhow::Result<Self> {
        if seed == 0 || seed > 16 {
            Err(anyhow!("Out of bounds seed value {}", seed))
        } else {
            Ok(Seed(seed))
        }
    }
}

/// Representation of teams in a matchup (basicially a more readable boolean indicator)
#[derive(Debug, Copy, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum MatchupInd {
    Team1,
    Team2,
}

impl MatchupInd {
    pub fn to_ind(self) -> usize {
        match self {
            Self::Team1 => 0,
            Self::Team2 => 1,
        }
    }
}

/// One matchup in a round
#[derive(Debug, Copy, Clone, Default)]
pub struct Matchup<'a> {
    /// Teams playing in this matchup, None if not determined yet
    teams: [Option<&'a str>; 2],
    /// Who won the matchup, None if not complete
    winner: Option<MatchupInd>,
}

impl<'a> Matchup<'a> {
    /// Get the competing teams, should only be called when both teams exist
    pub fn teams(&self) -> [String; 2] {
        [
            self.teams[0].unwrap().to_string(),
            self.teams[1].unwrap().to_string(),
        ]
    }

    /// Set the winner of this matchup
    pub fn set_winner(&mut self, winner: MatchupInd) {
        self.winner = Some(winner);
    }

    /// Return the winner of this matchup. Panics if the winner was not set
    pub fn winner(&self) -> &'a str {
        self.teams[self.winner.unwrap().to_ind()].unwrap()
    }

    /// Include a team in this matchup. Must have space for another team
    fn add_team(&mut self, name: &'a str) -> &mut Self {
        if self.teams[0].is_none() {
            self.teams[0] = Some(name)
        } else if self.teams[1].is_none() {
            self.teams[1] = Some(name)
        } else {
            panic!("Both teams already set!");
        }
        self
    }

    fn team_won(&self, team: MatchupInd) -> bool {
        self.winner == Some(team)
    }

    fn get_team_display(&self, ind: MatchupInd) -> ColoredString {
        let name = self.teams[ind.to_ind()].unwrap_or("___");
        if self.team_won(ind) {
            name.green()
        } else {
            name.red()
        }
    }
}

impl<'a> Display for Matchup<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} vs {}",
            self.get_team_display(MatchupInd::Team1),
            self.get_team_display(MatchupInd::Team2)
        )
    }
}

/// Tournament round
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum RoundKind {
    /// Play-in round
    PlayIn,
    /// Round 1, 2, etc
    Round(usize),
}

impl Display for RoundKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayIn => write!(f, "Play-in"),
            Self::Round(round) => write!(f, "Round {}", round),
        }
    }
}

/// Round in a tournament
pub struct Round<'a> {
    /// What round this is
    pub round: RoundKind,
    /// The matchups in this round
    pub matchups: Vec<Matchup<'a>>,
}

impl<'a> Round<'a> {
    /// Initialize an empty normal round
    pub fn new(round: usize) -> Self {
        let num_matchups = 2_usize.pow((6 - round) as u32);
        let matchups = vec![Matchup::default(); num_matchups];
        Self {
            round: RoundKind::Round(round),
            matchups,
        }
    }

    /// Initialize a round from a complete matchup set
    pub fn with_matchups(round: usize, matchups: Vec<Matchup<'a>>) -> Self {
        Self {
            round: RoundKind::Round(round),
            matchups,
        }
    }
}

impl<'a> Display for Round<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{}\n",
            self.round,
            self.matchups
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

/// Matchup number for a given seed in their region going top to bottom
fn matchup_ind(mut seed: u8) -> usize {
    if seed > 8 {
        seed = 17 - seed;
    }
    match seed {
        1 => 0,
        8 => 1,
        5 => 2,
        4 => 3,
        6 => 4,
        3 => 5,
        7 => 6,
        2 => 7,
        _ => unreachable!(),
    }
}

/// A complete tournament
pub struct Tournament<'a> {
    /// All rounds played
    pub rounds: Vec<Round<'a>>,
}

impl<'a> Tournament<'a> {
    /// Initialize from a list of teams. The first round will be set using these teams
    pub fn new(teams: &'a [Team]) -> Self {
        let mut round1 = Round::new(1);
        for team in teams {
            let seed = team.seed.0;
            let matchup_ind = matchup_ind(seed);
            let matchup = round1
                .matchups
                .get_mut(matchup_ind + 8 * team.region.to_ind())
                .unwrap();
            matchup.add_team(team.name());
        }
        Self {
            rounds: vec![round1],
        }
    }

    /// Construct the next round of matchups from a completed previous round
    pub fn initialize_next_round(&mut self) {
        let prev_winners = self
            .rounds
            .last()
            .unwrap()
            .matchups
            .iter()
            .map(|matchup| matchup.winner())
            .collect::<Vec<_>>();

        let matchups = (0..(prev_winners.len() / 2))
            .map(|i| Matchup {
                teams: [Some(prev_winners[2 * i]), Some(prev_winners[2 * i + 1])],
                winner: None,
            })
            .collect();
        let round = Round::with_matchups(self.rounds.len() + 1, matchups);
        self.rounds.push(round);
    }
}

impl<'a> Display for Tournament<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for round in &self.rounds {
            write!(f, "{}", round)?;
        }
        Ok(())
    }
}
