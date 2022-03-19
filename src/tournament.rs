use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;

use anyhow::anyhow;
use colored::*;
use serde::{Deserialize, Serialize};

use crate::teams::{construct_html_name, Team};

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
#[derive(Debug, Clone, Default)]
pub struct Matchup {
    /// Teams playing in this matchup, None if not determined yet
    teams: [Option<String>; 2],
    /// Who won the matchup, None if not complete
    winner: Option<MatchupInd>,
    /// What # matchup this is in the round (used so we know where to advance the winner to)
    index: usize,
}

impl Matchup {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            ..Default::default()
        }
    }
    /// Get the competing teams, should only be called when both teams exist
    pub fn teams(&self) -> [String; 2] {
        [
            self.teams[0].as_ref().unwrap().clone(),
            self.teams[1].as_ref().unwrap().clone(),
        ]
    }

    /// Set the winner of this matchup
    pub fn set_winner(&mut self, winner: MatchupInd) {
        self.winner = Some(winner);
    }

    pub fn set_winning_team(&mut self, team: &str) {
        if self.is_team_ind(team, MatchupInd::Team1) {
            self.set_winner(MatchupInd::Team1);
        } else {
            self.set_winner(MatchupInd::Team2);
        }
    }

    pub fn includes_team(&self, team: &str) -> bool {
        self.is_team_ind(team, MatchupInd::Team1) || self.is_team_ind(team, MatchupInd::Team2)
    }

    fn is_team_ind(&self, team: &str, ind: MatchupInd) -> bool {
        self.teams[ind.to_ind()]
            .as_ref()
            .map(|t| t == team)
            .unwrap_or_default()
    }

    pub fn completed(&self) -> bool {
        self.winner.is_some()
    }

    /// Include a team in this matchup. Must have space for another team
    fn add_team(&mut self, name: &str) -> &mut Self {
        if self.teams[0].is_none() {
            self.teams[0] = Some(name.to_string())
        } else if self.teams[1].is_none() {
            self.teams[1] = Some(name.to_string())
        } else {
            panic!("Both teams already set!");
        }
        self
    }

    fn team_won(&self, team: MatchupInd) -> bool {
        self.winner == Some(team)
    }

    fn get_team_display(&self, ind: MatchupInd) -> ColoredString {
        let name = self.teams[ind.to_ind()].as_deref().unwrap_or("___");
        if !self.completed() {
            name.normal()
        } else if self.team_won(ind) {
            name.green()
        } else {
            name.red()
        }
    }
}

impl Display for Matchup {
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
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum RoundKind {
    /// Play-in round
    PlayIn,
    /// Round 1, 2, etc
    Round(usize),
}

impl RoundKind {
    pub fn next_round(&self) -> Option<Self> {
        match self {
            RoundKind::PlayIn => Some(RoundKind::Round(1)),
            RoundKind::Round(r) if r < &6 => Some(RoundKind::Round(r + 1)),
            _ => None,
        }
    }

    pub fn matchup_count(&self) -> usize {
        match self {
            RoundKind::PlayIn => 4,
            RoundKind::Round(round) => 2_usize.pow((6 - round) as u32),
        }
    }
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
pub struct Round {
    /// What round this is
    pub round: RoundKind,
    /// The matchups in this round
    pub matchups: Vec<Matchup>,
}

impl Round {
    /// Initialize an empty normal round
    pub fn empty(round: usize) -> Self {
        let round = RoundKind::Round(round);
        let num_matchups = round.matchup_count();
        let matchups = (0..num_matchups).map(Matchup::new).collect();
        Self { round, matchups }
    }

    pub fn add_team_to_matchup(&mut self, team: &str, ind: usize) {
        self.matchups[ind].add_team(team);
    }

    pub fn new_round1(teams: &mut [Team]) -> Self {
        teams.sort_by_key(|team| team.seed.0 as usize + team.region.to_ind() * 16);
        let mut round = Self::empty(1);
        debug_assert_eq!(round.round.matchup_count() * 2, teams.len());

        for team in teams {
            let matchup_ind = matchup_ind(team.seed.0) + 8 * team.region.to_ind();
            round.add_team_to_matchup(team.name(), matchup_ind);
        }
        round
    }

    pub fn get_matchup_with_team_mut(&mut self, team: &str) -> &mut Matchup {
        for matchup in &mut self.matchups {
            if matchup.includes_team(team) {
                return matchup;
            }
        }
        panic!("Team {} not found", team);
    }

    pub fn get_matchup_with_team(&self, team: &str) -> &Matchup {
        for matchup in &self.matchups {
            if matchup.includes_team(team) {
                return matchup;
            }
        }
        panic!("Team {} not found", team);
    }
}

impl Display for Round {
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
pub struct Tournament {
    /// All rounds in this tournament
    pub rounds: HashMap<RoundKind, Round>,
}

impl Tournament {
    /// Initialize from a list of teams. The first round will be set using these teams
    pub fn new(teams: &mut [Team], current_results: HashMap<RoundKind, HashSet<String>>) -> Self {
        let mut rounds = HashMap::new();
        let round1 = Round::new_round1(teams);
        rounds.insert(round1.round, round1);

        for round_num in 2..=6 {
            let round = Round::empty(round_num);
            rounds.insert(round.round, round);
        }
        let mut tournament = Self { rounds };
        for round_kind in (1..=5).map(RoundKind::Round) {
            let mut teams_to_advance = vec![];
            if let Some(cur_teams) = current_results.get(&round_kind.next_round().unwrap()) {
                let round = &tournament.rounds[&round_kind];
                for matchup in &round.matchups {
                    for ind in [0, 1] {
                        if let Some(team) = &matchup.teams[ind] {
                            let html_name = construct_html_name(team);
                            if cur_teams.contains(&html_name) {
                                teams_to_advance.push(team.clone());
                            }
                        }
                    }
                }
            }

            for team in teams_to_advance {
                tournament.advance_team(&team, round_kind);
            }
        }
        tournament
    }

    pub fn advance_team(&mut self, team: &str, round: RoundKind) {
        self.get_round_mut(round)
            .get_matchup_with_team_mut(team)
            .set_winning_team(team);

        let matchup_ind = self.rounds[&round].get_matchup_with_team(team).index;

        let next_round_ind = matchup_ind / 2;
        if let Some(next_round) = round.next_round() {
            self.rounds.get_mut(&next_round).unwrap().matchups[next_round_ind].add_team(team);
        }
    }

    pub fn get_round_mut(&mut self, round: RoundKind) -> &mut Round {
        self.rounds.get_mut(&round).unwrap()
    }
}

impl Display for Tournament {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for round_num in 1..=6 {
            write!(f, "{}", self.rounds[&RoundKind::Round(round_num)])?;
        }
        Ok(())
    }
}
