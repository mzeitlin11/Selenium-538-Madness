use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::str::FromStr;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use thirtyfour::{By, WebDriver};

use crate::tournament::{Region, Seed};
use crate::URL;

const TEAMS_PATH_538: &str = "teams.json";

/// A team playing in the tournament
#[derive(Debug, Deserialize, Serialize)]
pub struct Team {
    /// Name as classified by 538
    name: String,
    /// Starting region
    pub region: Region,
    /// Seed in the tournament
    pub seed: Seed,
}

impl Team {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// Scrape the 538 teams table and write the participating teams to a file. Must use a
/// 538 source so that the names match to naming in HTML classes by 538.
pub async fn write_teams(driver: &WebDriver) -> anyhow::Result<()> {
    driver.get(URL).await?;
    let table = driver
        .find_elements(By::Css("#team-table tbody tr"))
        .await?;
    let mut teams = vec![];
    for team in table {
        let name_seed_text = team
            .find_element(By::ClassName("team-name"))
            .await?
            .inner_html()
            .await?;
        let mut name_seed = name_seed_text.split(" <span>");
        let name = name_seed.next().context("No team name found")?;

        let seed = name_seed
            .next()
            .context("No seed found")?
            .strip_suffix("</span>")
            .context("Unexpected line structure")?
            .parse()?;
        let region = team
            .find_element(By::ClassName("region"))
            .await?
            .inner_html()
            .await?;
        let team = Team {
            name: name.to_string(),
            region: Region::from_str(&region)?,
            seed: Seed::new(seed)?,
        };
        log::info!("Found team {}", name);
        teams.push(team);
    }
    let writer = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(TEAMS_PATH_538)?,
    );
    serde_json::to_writer_pretty(writer, &teams)?;
    Ok(())
}

/// Load 538 tournament team information written to file
pub fn load_teams() -> anyhow::Result<Vec<Team>> {
    let reader = BufReader::new(File::open(TEAMS_PATH_538)?);
    Ok(serde_json::from_reader(reader)?)
}
