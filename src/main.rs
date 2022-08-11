//! CLI tool for deleting your twitter activities
//! This is inspired by Delete Them All(a.k.a. 黒歴史クリーナー)
use anyhow::anyhow;
use env_logger::Env;
use log::error;
use std::path::PathBuf;
use structopt::StructOpt;
use twitter_client::TwitterClient;
mod cli;
mod dta_app;
mod twitter_client;
mod twitter_object;

use cli::{Action::*, CommandLineArgs};

/// Entrypoint Function
///
/// It will use the following environment variables (It is effective only the build time)
/// * `DTA4HANA_LOG_LEVEL` Log level setting e.g. `DTA4HANA_LOG_LEVEL=dta4hana=debug`
/// * `DTA4HANA_B` Bearer Token, it will be used for retrieving the user id and login process
/// * `DTA4HANA_CK` Consumer Key, it will be used for calling Twitter API as app
/// * `DTA4HANA_CS` Consumer Secret, it will be used for calling Twitter API as app
fn main() -> anyhow::Result<()> {
    let env = Env::default().filter_or("DTA4HANA_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    // Twitter Client初期化用のKeyなど, 定義がない場合は実行時エラーにする
    let api_key = option_env!("DTA4HANA_B");
    let consumer_key = option_env!("DTA4HANA_CK");
    let consumer_secret = option_env!("DTA4HANA_CS");

    if api_key.is_none() || consumer_key.is_none() || consumer_secret.is_none() {
        error!(
            "Please confirm the following environment values are defined: {}, {}, {}",
            "DTA4HANA_B", "DTA4HANA_CK", "DTA4HANA_CS"
        );
    }

    let api_key = api_key.unwrap().to_string();
    let consumer_key = consumer_key.unwrap().to_string();
    let consumer_secret = consumer_secret.unwrap().to_string();

    let CommandLineArgs {
        action,
        config_file,
    } = CommandLineArgs::from_args();

    let config_file = config_file
        .or_else(find_default_config_file)
        .ok_or(anyhow!("Failed to find config file."))?;

    let tw_client: TwitterClient =
        dta_app::init_client(api_key, consumer_key, consumer_secret, &config_file)?;

    match action {
        Delete { since, until } => dta_app::delete_tweets(&tw_client, since, until),
        Fetch { since, until } => dta_app::fetch_tweets(&tw_client, since, until),
        Login => dta_app::login(&tw_client, &config_file),
        Unlike => dta_app::unlike_likes(&tw_client),
    }?;
    Ok(())
}

/// Get the default path for storing user credential as a file
/// It assumes you have write permission in your home dir
fn find_default_config_file() -> Option<PathBuf> {
    let default_path = ".dta4hana.json";
    home::home_dir().map(|mut path| {
        path.push(default_path);
        path
    })
}

#[cfg(test)]
mod tests {
    use crate::{dta_app, find_default_config_file, twitter_client::TwitterClient};

    #[test]
    #[ignore]
    fn delete_tweets() {
        let api_key = option_env!("DTA4HANA_B").unwrap().to_string();
        let consumer_key = option_env!("DTA4HANA_CK").unwrap().to_string();
        let consumer_secret = option_env!("DTA4HANA_CS").unwrap().to_string();

        let tw_client: TwitterClient = dta_app::init_client(
            api_key,
            consumer_key,
            consumer_secret,
            &find_default_config_file().unwrap(),
        )
        .unwrap();
        let result = dta_app::delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[ignore]
    fn unlike_likes() {
        let api_key = option_env!("DTA4HANA_B").unwrap().to_string();
        let consumer_key = option_env!("DTA4HANA_CK").unwrap().to_string();
        let consumer_secret = option_env!("DTA4HANA_CS").unwrap().to_string();

        let tw_client: TwitterClient = dta_app::init_client(
            api_key,
            consumer_key,
            consumer_secret,
            &find_default_config_file().unwrap(),
        )
        .unwrap();
        let result = dta_app::unlike_likes(&tw_client);
        assert_eq!(result.is_ok(), true);
    }
}
