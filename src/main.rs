//! CLI tool for deleting your twitter activities
//! This is inspired by Delete Them All(a.k.a. 黒歴史クリーナー)
use anyhow::{anyhow, Error};
use env_logger::Env;
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
    let api_key = match option_env!("DTA4HANA_B") {
        Some(env) => env.to_string(),
        None => return Err(anyhow::anyhow!("No value is defined in {}", "DTA4HANA_B")),
    };
    let consumer_key = match option_env!("DTA4HANA_CK") {
        Some(env) => env.to_string(),
        None => return Err(anyhow::anyhow!("No value is defined in {}", "DTA4HANA_CK")),
    };
    let consumer_secret = match option_env!("DTA4HANA_CS") {
        Some(env) => env.to_string(),
        None => return Err(anyhow::anyhow!("No value is defined in {}", "DTA4HANA_CS")),
    };

    let CommandLineArgs {
        action,
        config_file,
    } = CommandLineArgs::from_args();

    let config_file = match config_file {
        Some(config_file) => config_file,
        None => find_default_config_file()?,
    };

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
fn find_default_config_file() -> Result<PathBuf, Error> {
    let default_path = ".dta4hana.json";
    match home::home_dir() {
        Some(mut home_dir) => {
            home_dir.push(default_path);
            Ok(home_dir)
        }
        None => Err(anyhow!("Failed to load home dir")),
    }
}

#[cfg(test)]
mod tests {
    use crate::{dta_app, find_default_config_file, twitter_client::TwitterClient};

    #[test]
    #[ignore]
    fn delete_tweets() {
        let api_key = match option_env!("DTA4HANA_B") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };
        let consumer_key = match option_env!("DTA4HANA_CK") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };
        let consumer_secret = match option_env!("DTA4HANA_CS") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };

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
        let api_key = match option_env!("DTA4HANA_B") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };
        let consumer_key = match option_env!("DTA4HANA_CK") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };
        let consumer_secret = match option_env!("DTA4HANA_CS") {
            Some(env) => env.to_string(),
            None => panic!(), 
        };

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
