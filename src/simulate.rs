use crate::teams::load_teams;
use crate::tournament::{MatchupInd, Tournament};
use crate::URL;
use std::time::Duration;
use thirtyfour::{By, WebDriver, WebElement};

fn construct_html_name(name: &str) -> String {
    name.chars()
        .filter(|&c| c == '-' || c.is_alphabetic())
        .collect()
}

pub async fn simulate(driver: &WebDriver) -> anyhow::Result<()> {
    driver.get(URL).await?;
    let html = driver
        .find_element(By::Css(".bracket-container svg g g.nodes"))
        .await?;
    html.scroll_into_view().await?;
    let mut teams = load_teams()?;
    teams.sort_by_key(|team| team.seed.0 as usize + team.region.to_ind() * 16);
    teams.dedup_by(|team1, team2| team1.seed == team2.seed && team1.region == team2.region);
    let mut tournament = Tournament::new(&teams);

    loop {
        let round_num = tournament.rounds.len();
        let curr_round = tournament.rounds.last_mut().unwrap();

        for matchup in &mut curr_round.matchups {
            matchup.set_winner(MatchupInd::Team1);
            let team = matchup.winner();
            let node = get_team_node(driver, team, round_num).await?;
            // hover_node(&node, driver).await?;
            click_node(&node, driver).await?;
        }

        if curr_round.matchups.len() == 1 {
            break;
        }
        tournament.initialize_next_round();
    }
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(())
}

async fn hover_node<'a>(ele: &'a WebElement<'a>, driver: &'a WebDriver) -> anyhow::Result<()> {
    driver
        .action_chain()
        .move_to_element_center(&ele)
        .perform()
        .await?;
    Ok(())
}

async fn click_node<'a>(ele: &'a WebElement<'a>, driver: &'a WebDriver) -> anyhow::Result<()> {
    driver
        .action_chain()
        .move_to_element_center(&ele)
        .click()
        .perform()
        .await?;
    Ok(())
}

async fn get_team_node<'a>(
    driver: &'a WebDriver,
    team: &str,
    round: usize,
) -> anyhow::Result<WebElement<'a>> {
    Ok(driver
        .find_element(By::Id(&format!(
            "node-{}-{}",
            construct_html_name(team),
            7 - round
        )))
        .await?)
}
