use anyhow::anyhow;
use env_logger::Env;
use std::path::PathBuf;
use structopt::StructOpt;
use twitter_client::TwitterClient;
mod cli;
mod dta_app;
mod twitter_client;
mod twitter_object;

use cli::{Action::*, CommandLineArgs};

fn main() -> anyhow::Result<()> {
    let env = Env::default().filter_or("DTA4HANA_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let CommandLineArgs {
        action,
        config_file,
    } = CommandLineArgs::from_args();

    let config_file = config_file
        .or_else(find_default_config_file)
        .ok_or(anyhow!("Failed to find config file."))?;

    let tw_client: TwitterClient = dta_app::init_client(&config_file)?;

    match action {
        Delete { since, until } => dta_app::delete_tweets(&tw_client, since, until),
        Fetch { since, until } => dta_app::fetch_tweets(&tw_client, since, until),
        Login => dta_app::login(&tw_client, &config_file),
        Unlike => dta_app::unlike_likes(&tw_client),
    }?;
    Ok(())
}

fn find_default_config_file() -> Option<PathBuf> {
    home::home_dir().map(|mut path| {
        path.push(".dta4hana.json");
        path
    })
}

#[cfg(test)]
mod tests {
    use crate::{dta_app, find_default_config_file, twitter_client::TwitterClient};

    #[test]
    #[ignore]
    fn delete_tweets() {
        let tw_client: TwitterClient =
            dta_app::init_client(&find_default_config_file().unwrap()).unwrap();
        let result = dta_app::delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[ignore]
    fn unlike_likes() {
        let tw_client: TwitterClient =
            dta_app::init_client(&find_default_config_file().unwrap()).unwrap();
        let result = dta_app::unlike_likes(&tw_client);
        assert_eq!(result.is_ok(), true);
    }
}
