use crate::teams::Team;
use anyhow::anyhow;
use rand::random;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum Region {
    West,
    South,
    Midwest,
    East,
}

impl Region {
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

#[derive(Debug, Copy, Clone, Default)]
pub struct Matchup<'a> {
    teams: [Option<&'a str>; 2],
    winner: Option<MatchupInd>,
}

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

impl<'a> Matchup<'a> {
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

    pub fn set_winner(&mut self, winner: MatchupInd) {
        self.winner = Some(winner);
    }

    pub fn winner(&self) -> &'a str {
        self.teams[self.winner.unwrap().to_ind()].unwrap()
    }

    fn team_won(&self, team: MatchupInd) -> bool {
        self.winner == Some(team)
    }

    fn get_team_display(&self, ind: MatchupInd) -> String {
        let name = self.teams[ind.to_ind()].unwrap_or("___");
        format!("{} {}", name, if self.team_won(ind) { "(won)" } else { "" })
    }
}

impl<'a> Display for Matchup<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} vs {}\n",
            self.get_team_display(MatchupInd::Team1),
            self.get_team_display(MatchupInd::Team2)
        )
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum RoundKind {
    PlayIn,
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

pub struct Round<'a> {
    pub round: RoundKind,
    pub matchups: Vec<Matchup<'a>>,
}

impl<'a> Round<'a> {
    pub fn new(round: usize) -> Self {
        let num_matchups = 2_usize.pow((6 - round) as u32);
        let matchups = vec![Matchup::default(); num_matchups];
        Self {
            round: RoundKind::Round(round),
            matchups,
        }
    }

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

pub struct Tournament<'a> {
    pub rounds: Vec<Round<'a>>,
}

impl<'a> Tournament<'a> {
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
