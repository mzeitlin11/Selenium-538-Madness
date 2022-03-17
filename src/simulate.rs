use std::collections::HashSet;

use anyhow::anyhow;
use rand::random;
use scraper::{Html, Selector};
use thirtyfour::{By, WebDriver, WebElement};

use crate::teams::load_teams;
use crate::tournament::{MatchupInd, Tournament};
use crate::URL;

/// Convert the 538 team name to an HTML-friendly name used in element classes
fn construct_html_name(name: &str) -> String {
    name.chars()
        .filter(|&c| c == '-' || c.is_alphabetic())
        .collect()
}

const PLAY_IN_LOSERS: [&str; 4] = ["Rutgers", "Wyoming", "Bryant", "TX A&amp;M-CC"];

pub async fn simulate(driver: &WebDriver) -> anyhow::Result<()> {
    driver.get(URL).await?;
    let mut teams = load_teams()?;
    teams.sort_by_key(|team| team.seed.0 as usize + team.region.to_ind() * 16);
    teams.retain(|team| !PLAY_IN_LOSERS.contains(&team.name()));
    let mut tournament = Tournament::new(&teams);

    loop {
        let round_num = tournament.rounds.len();
        let curr_round = tournament.rounds.last_mut().unwrap();

        for matchup in &mut curr_round.matchups {
            let teams = matchup.teams();
            let win_perc = get_win_percent(driver, &teams[0], round_num).await?;

            log::info!(
                "{} has a {}% to win against {}",
                teams[0],
                win_perc,
                teams[1]
            );

            if random::<f32>() < (win_perc as f32 / 100.) {
                matchup.set_winner(MatchupInd::Team1);
            } else {
                matchup.set_winner(MatchupInd::Team2);
            }
            let team = matchup.winner();
            log::info!("{} won!", team);
            click_team(driver, team, round_num).await?;
        }

        if curr_round.matchups.len() == 1 {
            break;
        }
        tournament.initialize_next_round();
    }
    log::info!("Tournament results: {}\n\n", tournament);
    Ok(())
}

/// Get the win% for this team in the given round. This requires 2 steps:
/// 1. Hover over the team node so that the HTML updates to include the win %
/// 2. Parse the HTML to extract the win %
async fn get_win_percent(driver: &WebDriver, team: &str, round_num: usize) -> anyhow::Result<u32> {
    let team = construct_html_name(team);
    let node = get_team_node(driver, &team, round_num).await?;
    hover_node(&node, driver).await?;
    let html = driver
        .find_element(By::Css("g.nodes"))
        .await?
        .inner_html()
        .await?;
    let parsed = Html::parse_fragment(&html);
    let css_selector = format!("text[depth=\"{}\"", 6 - round_num);
    let selector = Selector::parse(&css_selector).unwrap();
    for node in parsed.select(&selector) {
        // TODO: seems like there should be a more idiomatic way to use this Classes type
        if node
            .value()
            .classes
            .iter()
            .map(|c| c.to_string())
            .collect::<HashSet<_>>()
            .contains(&team)
        {
            let text = node.text().collect::<Vec<_>>();
            // We should have one text element here if we've found the win %
            if text.len() == 1 {
                return Ok(text[0].replace('%', "").parse()?);
            }
        }
    }
    Err(anyhow!("No win percentage found for {}", team))
}

/// Hover over the given node, used to expose up to date win percentages
async fn hover_node<'a>(ele: &'a WebElement<'a>, driver: &'a WebDriver) -> anyhow::Result<()> {
    driver
        .action_chain()
        .move_to_element_center(ele)
        .perform()
        .await?;
    Ok(())
}

/// Click the team node for the given round, which will advance the team
async fn click_team(driver: &WebDriver, team: &str, round_num: usize) -> anyhow::Result<()> {
    let team = construct_html_name(team);
    let node = get_team_node(driver, &team, round_num).await?;
    click_node(&node, driver).await?;
    Ok(())
}

/// Click the given element. Note that we use this utility for clicking an element that is not
/// clickable - for example the 538 team nodes are not clickable, so instead we move the
/// mouse to them and click such that the clickable element in the same location intercepts it.
async fn click_node<'a>(ele: &'a WebElement<'a>, driver: &'a WebDriver) -> anyhow::Result<()> {
    driver
        .action_chain()
        .move_to_element_center(ele)
        .click()
        .perform()
        .await?;
    driver.action_chain().reset_actions().await?;
    Ok(())
}

/// Get a node for this team in the given round. The name argument should already
/// be sanitized
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
