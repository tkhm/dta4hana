use anyhow::{Error, Result};
use log::debug;
use log::info;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::thread::sleep;
use url::Url;

use crate::twitter_client::TwitterClient;
use crate::twitter_client::TwitterClientTrait;
use crate::twitter_object::Tweet;

/// ローカルに保存したツイートを読み出す
/// 作業のResume用だが現状の想定使用メモリなどから照らしてオンメモリでの処理の方が望ましそうなため使用予定なし
fn collect_tweets(mut file: &File) -> Result<Vec<Tweet>> {
    file.seek(SeekFrom::Start(0))?; // Rewind the file before.
    let tweets = match serde_json::from_reader(file) {
        Ok(tweets) => tweets,
        Err(e) if e.is_eof() => Vec::new(),
        Err(e) => Err(e)?,
    };
    file.seek(SeekFrom::Start(0))?; // Rewind the file after.
    Ok(tweets)
}

pub fn delete_tweets(
    tw_client: &impl TwitterClientTrait,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    debug!("args: since={:?}, until={:?}", since, until);

    let result = tw_client.fetch_timeline(since, until);

    if result.is_err() {
        panic!()
    }
    let total_tweets_count = &result.as_ref().unwrap().len();
    let mut deleted_tweets_count = 0;
    info!("Start to delete {} tweets", total_tweets_count);
    for val in result.unwrap().iter() {
        let deleted = tw_client.delete_tweet(&val.id);
        if deleted != true {
            return Err(anyhow::anyhow!("Delete was failed with {:?}", &val.id));
        }
        deleted_tweets_count += 1;
        info!(
            "Deleted Id: {:?}, {} / {}",
            &val.id, deleted_tweets_count, total_tweets_count
        );
        // 早く投げすぎてブロックされることを防ぐため、インターバルを挟む
        let request_interval = std::time::Duration::from_secs(2);
        sleep(request_interval);
    }
    Ok({})
}

pub fn fetch_tweets(
    tw_client: &impl TwitterClientTrait,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    debug!("args: since={:?}, until={:?}", since, until);
    let result: Result<Vec<Tweet>, Error> = tw_client.fetch_timeline(since, until);
    for val in result.iter() {
        debug!("Got: {:?}", val);
    }
    Ok({})
}

pub fn unlike_likes(tw_client: &impl TwitterClientTrait) -> Result<()> {
    // TODO: Replace work_path
    use std::env;
    let mut work_path = env::temp_dir();
    work_path.push("dta4hana.work.json");
    let file: File = OpenOptions::new().read(true).open(&work_path)?;

    // Unlike対象IDを確保する
    let result: Vec<Tweet> = collect_tweets(&file)?;
    for val in result.iter() {
        let deleted = tw_client.delete_liked(&val.id);
        debug!("Id: {:?}", &val.id);
        if deleted != true {
            return Err(anyhow::anyhow!("Unlike was failed with {:?}", &val.id));
        }
    }
    Ok({})
}

pub fn init_client(config_path: &PathBuf) -> Result<TwitterClient, Error> {
    let tw_client = TwitterClient::new(Url::parse("https://api.twitter.com").unwrap());
    let tw_client = tw_client?.initiaize(&config_path);
    Ok(tw_client.unwrap())
}

pub fn login(tw_client: &impl TwitterClientTrait, config_path: &PathBuf) -> Result<()> {
    let _ = tw_client.login(&config_path);
    Ok({})
}

#[cfg(test)]
mod tests {
    use crate::{
        dta_app::{delete_tweets, unlike_likes},
        twitter_client::MockTwitterClientTrait,
    };

    #[test]
    fn delete_tweets_all() {
        // setup required
        let mut tw_client = MockTwitterClientTrait::default();
        tw_client.expect_delete_tweet().returning(|_| true);
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn delete_tweets_in_the_period() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: setup period config required
        tw_client.expect_delete_tweet().returning(|_| true);
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn delete_tweets_except_protected() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: setup protected config required
        tw_client.expect_delete_tweet().returning(|_| true);
        let result = delete_tweets(&tw_client, None, None);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[ignore]
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

    #[test]
    #[ignore]
    fn unlike_likes_in_the_period() {
        // TODO: setup required
        let mut tw_client = MockTwitterClientTrait::default();
        // TODO: setup period config required
        // TODO: modify here after implementation
        tw_client
            .expect_delete_liked()
            .returning(|_| unimplemented!());
        let result = unlike_likes(&tw_client);
        assert_eq!(result.is_ok(), true);
    }
}
