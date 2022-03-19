use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Context};
use rand::random;
use scraper::{Html, Selector};
use thirtyfour::{By, WebDriver, WebElement};

use crate::teams::{construct_html_name, load_teams};
use crate::tournament::{RoundKind, Tournament};
use crate::URL;

/// For example, node-Kentucky-6 -> ("Kentucky", 6)
fn extract_team_round_from_id(id: &str) -> anyhow::Result<(String, RoundKind)> {
    let (left, seed_str) = id
        .rsplit_once('-')
        .ok_or_else(|| anyhow!("Unexpected format"))?;
    let (_, team) = left
        .split_once('-')
        .ok_or_else(|| anyhow!("Unexpected format"))?;
    let round = RoundKind::Round(7 - seed_str.parse::<usize>()?);
    Ok((team.to_string(), round))
}

/// Simulate the tournament using 538 predictions from the current bracket state.
pub async fn simulate(driver: &WebDriver) -> anyhow::Result<()> {
    driver.get(URL).await?;

    let current_teams = get_current_teams(driver).await?;
    let round1_teams = current_teams.get(&RoundKind::Round(1)).unwrap();
    let mut teams = load_teams()?;

    // Filter out teams who lost in the play-in. TODO: actually handle the play-in
    teams.retain(|team| {
        let html_name = construct_html_name(team.name());
        round1_teams.contains(&html_name)
    });
    let mut tournament = Tournament::new(&mut teams, current_teams);

    for round_num in 1..=6 {
        let mut winning_teams = vec![];
        let round_kind = RoundKind::Round(round_num);
        let curr_round = tournament.get_round_mut(round_kind);

        for matchup in &mut curr_round.matchups {
            if matchup.completed() {
                continue;
            }
            let teams = matchup.teams();
            let win_perc = get_win_percent(driver, &teams[0], round_num)
                .await
                .with_context(|| {
                    format!(
                        "Could not find win percentage for {} vs {}",
                        teams[0], teams[1]
                    )
                })?;

            log::info!(
                "{} has a {}% to win against {}",
                teams[0],
                win_perc,
                teams[1]
            );

            let winning_team = if random::<f32>() < (win_perc as f32 / 100.) {
                &teams[0]
            } else {
                &teams[1]
            };
            winning_teams.push(winning_team.clone());
            log::info!("{} won!", winning_team);
            click_team(driver, winning_team, round_num).await?;
        }

        for team in &winning_teams {
            tournament.advance_team(team, round_kind);
        }
    }
    log::info!("Tournament results: {}\n\n", tournament);
    Ok(())
}

/// Get a map of round to team currently advanced to that round
async fn get_current_teams(
    driver: &WebDriver,
) -> anyhow::Result<HashMap<RoundKind, HashSet<String>>> {
    let html = driver
        .find_element(By::Css("g.nodes"))
        .await?
        .inner_html()
        .await?;
    let parsed = Html::parse_fragment(&html);
    let selector = Selector::parse("g.node").unwrap();
    let mut res: HashMap<_, HashSet<_>> = HashMap::new();
    for node in parsed.select(&selector) {
        let id = node
            .value()
            .id
            .as_ref()
            .map(|a| a.to_string())
            .unwrap_or_default();
        if let Ok((team, round)) = extract_team_round_from_id(&id) {
            match res.entry(round) {
                Entry::Occupied(mut teams) => {
                    teams.get_mut().insert(team);
                }
                Entry::Vacant(entry) => {
                    let mut teams = HashSet::new();
                    teams.insert(team);
                    entry.insert(teams);
                }
            }
        }
    }
    Ok(res)
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
                return Ok(match text[0] {
                    ">99%" => 100,
                    "<1%" => 0,
                    t => t.replace('%', "").parse()?,
                });
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
