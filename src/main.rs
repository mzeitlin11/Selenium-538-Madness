mod simulate;
mod teams;
mod tournament;

use crate::teams::write_teams;
use clap::Parser;
use thirtyfour::{DesiredCapabilities, WebDriver};

const SELENIUM_SERVER_URL: &str = "http://localhost:4444/wd/hub";
const URL: &str = "https://projects.fivethirtyeight.com/2022-march-madness-predictions/";

#[derive(PartialEq, Debug, Copy, Clone, clap::ArgEnum)]
pub enum Task {
    WriteTeamsTable,
    Simulate,
}

#[derive(Parser)]
struct Opts {
    #[clap(arg_enum)]
    task: Task,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let args = Opts::parse();

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new(SELENIUM_SERVER_URL, &caps).await?;
    let res = match args.task {
        Task::WriteTeamsTable => write_teams(&driver).await,
        Task::Simulate => simulate::simulate(&driver).await,
    };

    if let Err(e) = res {
        log::error!("{}", e);
    }
    driver.quit().await?;

    Ok(())
}
