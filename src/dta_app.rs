//! App module and interface between CLI and Twitter Client/APIs
//! pub methods are expected to call from [`#main`]
#![allow(unused_assignments)]
use anyhow::{Error, Result};
use log::debug;
use log::info;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::result::Result::Ok;
use std::thread::sleep;

use crate::twitter_client::TwitterAppUserCredential;
use crate::twitter_client::TwitterClient;
use crate::twitter_client::TwitterClientTrait;
use crate::twitter_object::Tweet;

/// Delete the tweets
///
/// It can delete tweets only one by one, but accepts to receive desired target periods and try to repeat the deletion
/// In here, get target 100 tweets, delete it and repeat until the end(or API limits)
/// * tw_client: Twitter Client with valid credentials are required
/// * since: the first date of getting tweets e.g. 2022-01-01
///   It will be attached time and timezone after that date like 2022-01-01T00:00:00Z
/// * until: the last date of getting tweets e.g. 2022-12-31
///   It will be attached time and timezone after that date like 2022-12-31T00:00:00Z
pub fn delete_tweets(
    tw_client: &impl TwitterClientTrait,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    debug!("args: since={:?}, until={:?}", &since, &until);

    info!("We can't delete tweets all at once due to API limitation and current implementations. It will repeat your delete until it becomes 0. (or API call limits)");

    let mut is_continued = true;
    while is_continued {
        let result = match tw_client.fetch_timeline(since.clone(), until.clone()) {
            Ok(result) => result,
            Err(_) => {
                is_continued = false;
                info!("Looks nothing to delete. Exit the execution.");
                break;
            }
        };

        let total_tweets_count = &result.len();
        if total_tweets_count.eq(&0) {
            is_continued = false;
            info!("Looks nothing to delete. Exit the execution.");
            break;
        }

        let mut deleted_tweets_count = 0;
        info!("Start to delete {} tweets", total_tweets_count);
        for val in result {
            let deleted = tw_client.delete_tweet(&val.id);
            if deleted.is_err() {
                return Err(anyhow::anyhow!("Delete was failed with {:?}", &val.id));
            }
            deleted_tweets_count += 1;
            info!(
                "Deleted Id: {:?}, {} / {}",
                &val.id, deleted_tweets_count, total_tweets_count
            );
            // 早く投げすぎてブロックされることを防ぐため、インターバルを挟む
            let request_interval = std::time::Duration::from_millis(500);
            sleep(request_interval);
        }
        info!("Finished the round of deletion! (will continue to delete in the next round if necessary)")
    }
    Ok(())
}

/// Fetch the tweets, but actually it is typically for the test purpose and not intended to use by the user
/// At the moment, flush got tweets(only id + metrics) for debugging purpose
///  
/// * tw_client: Twitter Client with valid credentials are required
/// * since: the first date of getting tweets e.g. 2022-01-01
///   It will be attached time and timezone after that date like 2022-01-01T00:00:00Z
/// * until: the last date of getting tweets e.g. 2022-12-31
///   It will be attached time and timezone after that date like 2022-12-31T00:00:00Z
pub fn fetch_tweets(
    tw_client: &impl TwitterClientTrait,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    debug!("args: since={:?}, until={:?}", since, until);

    let result = match tw_client.fetch_timeline(since, until) {
        Ok(result) => result,
        Err(_) => return Err(anyhow::anyhow!("Failed or nothing to fetch the tweets")),
    };

    for val in &result {
        debug!("id: {}, created_at: {}", &val.id, &val.created_at);
    }

    // TODO: Replace work_path
    let mut work_path = env::temp_dir();
    work_path.push("dta4hana.work.json");

    if work_path.exists() {
        debug!("Work file {} will be overwritten", work_path.display());
    } else {
        debug!("Work file {} will be created", work_path.display());
    }
    let mut file = File::create(work_path)?;
    serde_json::to_writer(&mut file, &result)?;
    Ok(())
}

/// Initalize Twitter Client
///
/// If there is no credential file then it will ask you to proceed login
/// And if you have a credential then it will load it and will not ask you to re-login
/// * api_key: Bearder Token
/// * consumer_key: Consumer Key
/// * consumer_secret: Consumer Secret
/// * config_path: path to the user credential file
pub fn init_client(
    api_key: String,
    consumer_key: String,
    consumer_secret: String,
    config_path: &PathBuf,
) -> Result<TwitterClient, Error> {
    let loaded_user_cred = match load_app_user_credential(config_path) {
        Ok(user_cred) => Some(user_cred),
        Err(_) => None,
    };
    let mut tw_client: TwitterClient;
    if loaded_user_cred.is_some() {
        tw_client = TwitterClient::new(api_key, consumer_key, consumer_secret, loaded_user_cred);
    } else {
        tw_client = TwitterClient::new(api_key, consumer_key, consumer_secret, loaded_user_cred);

        let user_cred = login_and_store(&tw_client, config_path)?;
        tw_client = tw_client.init_user_cred(user_cred)?;
    };

    Ok(tw_client)
}

/// Login
/// At the moment, for aligning the inferface in [`#main`] purpose, it wraps [`login_and_store()`]
/// * tw_client: Twitter Client, but in here, no valid user credential is needed
/// * config_path: path of storing the user credential after login
pub fn login(tw_client: &impl TwitterClientTrait, config_path: &PathBuf) -> Result<()> {
    let _ = login_and_store(tw_client, config_path);
    info!("Login process was completed.");
    Ok(())
}

/// Unlike your liked tweets
///
/// It can unlike tweets only one by one, but try to repeat the unlike.
/// In here, get target 100 tweets, unlike it and repeat until the end(or API limits)
pub fn unlike_likes(tw_client: &impl TwitterClientTrait) -> Result<()> {
    info!("We can't unlike tweets all at once due to API limitation and current implementations. It will repeat your unlike until it becomes 0. (or API call limits)");

    let mut is_continued = true;
    while is_continued {
        let result = match tw_client.fetch_likes() {
            Ok(result) => result,
            Err(_) => {
                is_continued = false;
                info!("Looks nothing to unlike. Exit the execution.");
                break;
            }
        };

        let total_tweets_count = &result.len();
        if total_tweets_count.eq(&0) {
            is_continued = false;
            info!("Looks nothing to unlike. Exit the execution.");
            break;
        }

        let mut unliked_tweets_count = 0;
        info!("Start to unlike {} tweets", total_tweets_count);
        for val in result {
            let deleted = tw_client.delete_liked(&val.id);
            unliked_tweets_count += 1;
            if deleted.is_ok() {
                info!(
                    "Unliked Id: {:?}, {} / {}",
                    &val.id, unliked_tweets_count, total_tweets_count
                );
            } else {
                // 削除されたツイートに対するUnlikeができないため, ErrよりもContinueする
                unliked_tweets_count += 1;
                info!(
                    "(Skipped) Id: {:?}, {} / {}",
                    &val.id, unliked_tweets_count, total_tweets_count
                );
            }
            // 早く投げすぎてブロックされることを防ぐため、インターバルを挟む
            let request_interval = std::time::Duration::from_millis(500);
            sleep(request_interval);
        }
        info!("Finished the round of unlike! (will continue to unlike in the next round if necessary)")
    }
    Ok(())
}

/// Load your tweets from the file
/// It is for the test/verification purpose
fn _collect_tweets(mut file: &File) -> Result<Vec<Tweet>> {
    file.seek(SeekFrom::Start(0))?; // Rewind the file before.
    let tweets = match serde_json::from_reader(file) {
        Ok(tweets) => tweets,
        Err(e) if e.is_eof() => Vec::new(),
        Err(e) => Err(e)?,
    };
    file.seek(SeekFrom::Start(0))?; // Rewind the file after.
    Ok(tweets)
}

/// Load user credential from the file
/// * config_path: path of the credential stored file
fn load_app_user_credential(config_path: &PathBuf) -> Result<TwitterAppUserCredential> {
    let mut file = OpenOptions::new().read(true).open(config_path)?;
    file.seek(SeekFrom::Start(0))?; // Rewind the file before.
    let loaded_config = match serde_json::from_reader(file) {
        Ok(loaded_config) => loaded_config,
        Err(e) => Err(e)?,
    };
    Ok(loaded_config)
}

/// Login and store the credential in the file
///
/// * tw_client: Twitter Client, but in here, no valid user credential is needed
/// * config_path: path of storing the user credential after login
fn login_and_store(
    tw_client: &impl TwitterClientTrait,
    config_path: &PathBuf,
) -> Result<TwitterAppUserCredential> {
    let user_cred = tw_client.login()?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(config_path)?;
    serde_json::to_writer(file, &user_cred)?;
    Ok(user_cred)
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use crate::{
        dta_app::{delete_tweets, unlike_likes},
        twitter_client::MockTwitterClientTrait,
    };

    #[test]
    fn delete_tweets_all() {
        // setup required
        let mut tw_client = MockTwitterClientTrait::default();
        tw_client
            .expect_fetch_timeline()
            .returning(|_, _| Ok(vec![]));
        tw_client.expect_delete_tweet().returning(|_| Ok(()));
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[ignore]
    fn delete_tweets_in_the_period() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: setup period config required
        tw_client
            .expect_fetch_timeline()
            .returning(|_, _| Ok(vec![]));
        tw_client.expect_delete_tweet().returning(|_| Ok(()));
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn delete_tweets_except_protected() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: setup protected config required
        tw_client
            .expect_fetch_timeline()
            .returning(|_, _| Ok(vec![]));
        tw_client.expect_delete_tweet().returning(|_| Ok(()));
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn unlike_likes_all() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: modify here after implementation
        tw_client
            .expect_delete_liked()
            .returning(|_| unimplemented!());
        let result = unlike_likes(&tw_client);
        assert_eq!(result.is_ok(), true);
    }
}
