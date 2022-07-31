use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Seek, SeekFrom};
use url::Url;
use uuid::Uuid;

use crate::twitter_object::{ResponseObject, Tweet, TweetCount, User};

pub struct TwitterClient {
    agent: ureq::Agent,
    server: Url,
    app_cred: TwitterAppCredential,
    user_cred: Option<TwitterAppUserCredential>,
    pub work_path: PathBuf,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterAppCredential {
    pub api_key: String,
    pub consumer_key: String,
    pub consumer_secret: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterAppUserCredential {
    pub username: String,
    pub id: String,
    pub oauth_token: String,
    pub oauth_token_secret: String,
}

fn load_app_user_credential(config_path: &PathBuf) -> Result<TwitterAppUserCredential> {
    let mut file = OpenOptions::new().read(true).open(&config_path)?;
    file.seek(SeekFrom::Start(0))?; // Rewind the file before.
    let loaded_config = match serde_json::from_reader(file) {
        Ok(loaded_config) => loaded_config,
        Err(e) => Err(e)?,
    };
    Ok(loaded_config)
}

#[cfg(test)]
use mockall::{automock, predicate::*};
#[cfg_attr(test, automock)]
pub trait TwitterClientTrait {
    fn new(server: Url) -> Result<TwitterClient>;
    fn initiaize(self, config_path: &PathBuf) -> Result<TwitterClient>;
    fn login(&self, config_path: &PathBuf) -> Result<TwitterAppUserCredential>;
    fn count_tweets(&self, since: Option<String>, until: Option<String>) -> u32;
    fn fetch_timeline(&self, since: Option<String>, until: Option<String>) -> Result<Vec<Tweet>>;
    fn delete_liked(&self, tweet_id_str: &str) -> bool;
    fn delete_tweet(&self, tweet_id_str: &str) -> bool;
}

impl TwitterClientTrait for TwitterClient {
    fn new(server: Url) -> Result<TwitterClient> {
        let agent: ureq::Agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(5))
            .timeout_write(Duration::from_secs(5))
            .build();

        let api_key = env!("DTA4HANA_B").to_string();
        let consumer_key = env!("DTA4HANA_CK").to_string();
        let consumer_secret = env!("DTA4HANA_CS").to_string();
        let app_cred = TwitterAppCredential {
            api_key,
            consumer_key,
            consumer_secret,
        };

        let mut work_path = env::temp_dir();
        work_path.push("dta4hana.work.json");

        Ok(TwitterClient {
            agent,
            server,
            app_cred,
            user_cred: None,
            work_path,
        })
    }

    fn initiaize(mut self, config_path: &PathBuf) -> Result<TwitterClient> {
        let loaded_user_cred: Result<TwitterAppUserCredential> =
            load_app_user_credential(&config_path);
        if loaded_user_cred.is_ok() {
            info!(
                "Load existing cred instance in {}",
                &config_path.to_str().unwrap()
            );
            self.user_cred.replace(loaded_user_cred.ok().unwrap());
        } else {
            info!(
                "Start login sequence and credential will be saved in {}",
                &config_path.to_str().unwrap()
            );
            let loaded_user_cred = self.login(&config_path);
            self.user_cred.replace(loaded_user_cred.ok().unwrap());
        }
        Ok(self)
    }

    fn login(&self, config_path: &PathBuf) -> Result<TwitterAppUserCredential> {
        // 情報がないのでログイン処理
        // ユーザーからの入力
        info!("Please input your Twitter username:");
        let mut username = String::new();
        std::io::stdin().read_line(&mut username).unwrap();

        let liveness_request = self
            .server
            .join(&format!("2/users/by/username/{}", username.trim()))?;
        let liveness_response = self
            .agent
            .request_url("GET", &liveness_request)
            .set(
                "Authorization",
                &format!("Bearer {}", self.app_cred.api_key),
            )
            .call()?;

        let user_object: ResponseObject<User> =
            serde_json::from_reader(liveness_response.into_reader())?;

        let user_id = user_object.data.id;

        info!("Your username and user id is confirmed.");

        let mut work_path = env::temp_dir();
        work_path.push("dta4hana.work.json");

        // トークンリクエスト
        let request_token_request = self.server.join(&format!(
            "oauth/request_token?oauth_consumer_key={}&oauth_callback=oob",
            self.app_cred.consumer_key
        ))?;
        let token_request_response = self
            .agent
            .request_url("POST", &request_token_request)
            .set(
                "Authorization",
                &format!("Bearer {}", self.app_cred.api_key),
            )
            .call()?;

        let result = token_request_response.into_string()?;
        let result_map: Vec<&str> = result.split('&').collect();

        // oauth_callback_confirmed, oauth_token, oauth_token_secret
        let mut request_token_keys: HashMap<String, String> = HashMap::new();
        for each in result_map {
            let each_line: Vec<&str> = each.split('=').collect();
            request_token_keys.insert(each_line[0].to_string(), each_line[1].to_string());
        }

        debug!("request_token_keys: {:?}", &request_token_keys);
        // 認証
        let authorize_request = self.server.join(&format!(
            "oauth/authorize?oauth_token={}",
            request_token_keys.get("oauth_token").unwrap()
        ))?;

        info!(
            "Please open this URL in your browser: {}",
            authorize_request.to_string()
        );

        // ユーザーからの入力
        info!("After authorize app, please input PIN number on the screen for complete the authorization process:");
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).unwrap();

        // 認証完了
        let access_token_request = self.server.join(&format!(
            "oauth/access_token?oauth_token={}&oauth_verifier={}",
            request_token_keys.get("oauth_token").unwrap(),
            s.trim()
        ))?;
        let access_token_response = self
            .agent
            .request_url("POST", &access_token_request)
            .call()?;

        let result = access_token_response.into_string()?;
        let result_map: Vec<&str> = result.split('&').collect();
        // oauth_token, oauth_token_secret, user_id, screen_name
        let mut access_token_keys: HashMap<String, String> = HashMap::new();
        for each in result_map {
            let each_line: Vec<&str> = each.split('=').collect();
            access_token_keys.insert(each_line[0].to_string(), each_line[1].to_string());
        }

        debug!("access_token_keys: {:?}", access_token_keys);

        let oauth_token = access_token_keys.get("oauth_token").unwrap().to_string();
        let oauth_token_secret = access_token_keys
            .get("oauth_token_secret")
            .unwrap()
            .to_string();
        let user_cred = TwitterAppUserCredential {
            username,
            id: user_id,
            oauth_token,
            oauth_token_secret,
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config_path)?;
        serde_json::to_writer(file, &user_cred)?;
        Ok(user_cred)
    }

    /// Tweet一覧を取得、だが、Academic usage onlyなのでこのエンドポイントは呼び出せない
    fn count_tweets(&self, mut since: Option<String>, mut until: Option<String>) -> u32 {
        info!("Get the target tweets counts");

        if since.is_some() {
            let mut since_date = String::new();
            since_date.push_str(since.as_ref().unwrap());
            since_date.push_str("T00:00:00Z");
            since = Some(since_date);
        }
        if until.is_some() {
            let mut until_date = String::new();
            until_date.push_str(until.as_ref().unwrap());
            until_date.push_str("T00:00:00Z");
            until = Some(until_date);
        }

        let oauth_nonce = Uuid::new_v4();
        let oauth_token = &self.user_cred.as_ref().unwrap().oauth_token;
        let oauth_token_secret = &self.user_cred.as_ref().unwrap().oauth_token_secret;
        let consumer_key = &self.app_cred.consumer_key;
        let consumer_secret = &self.app_cred.consumer_secret;

        let encoded_consumer_secret: String =
            url::form_urlencoded::byte_serialize(&consumer_secret.as_bytes()).collect();
        let encoded_oauth_token_secret: String =
            url::form_urlencoded::byte_serialize(&oauth_token_secret.as_bytes()).collect();
        let signagure_key = format!("{}&{}", encoded_consumer_secret, encoded_oauth_token_secret);

        // メソッドとURL以外のSignature Data構成要素特定
        let oauth_signature_method = "HMAC-SHA1";
        let oauth_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let oauth_version = "1.0";

        let tweet_count_request = self.server.join(&format!("2/tweets/counts/all")).unwrap();
        // TODO: つけるquery paramによってsignature data内で配置すべき位置が異なる、これを自動で実現可能な形にするべきだが現状はできていない
        // TODO: この冗長な記載はリクエストを投げる箇所でも同様になっている
        let signature_data: String;
        if since.is_some() && until.is_some() {
            let encoded_until: String =
                url::form_urlencoded::byte_serialize(&until.as_ref().unwrap().as_bytes()).collect();
            let encoded_since: String =
                url::form_urlencoded::byte_serialize(&since.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "end_time={}&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&start_time={}",
                &encoded_until,
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version,
                &encoded_since
            );
        } else if since.is_some() {
            let encoded_since: String =
                url::form_urlencoded::byte_serialize(&since.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&start_time={}",
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version,
                &encoded_since
            );
        } else if until.is_some() {
            let encoded_until: String =
                url::form_urlencoded::byte_serialize(&until.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "end_time={}&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}",
                &encoded_until,
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version
            );
        } else {
            signature_data = format!(
                "oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}",
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version
            );
        }

        // https://rust-lang-nursery.github.io/rust-cookbook/encoding/strings.html#percent-encode-a-string
        let request_method = String::from("GET");
        let encoded_request_target: String =
            url::form_urlencoded::byte_serialize(&tweet_count_request.as_str().as_bytes())
                .collect();
        let encoded_sigature_data: String =
            url::form_urlencoded::byte_serialize(&signature_data.as_bytes()).collect();
        let joined_signature_data = format!(
            "{}&{}&{}",
            request_method, encoded_request_target, encoded_sigature_data
        );
        debug!("sig data: {}", joined_signature_data);

        debug!("sig key: {}", &signagure_key);
        let hmac_digest =
            hmacsha1::hmac_sha1(&signagure_key.as_bytes(), &joined_signature_data.as_bytes());
        let signature = base64::encode(hmac_digest);
        let encoded_signature: String =
            url::form_urlencoded::byte_serialize(&signature.as_str().as_bytes()).collect();
        debug!("sig base64: {}", signature);

        let mut signed_tweet_count_request = self
            .agent
            .request_url(request_method.as_str(), &tweet_count_request)
            .set("Authorization", &format!(
                "OAuth oauth_consumer_key={},oauth_nonce={},oauth_signature={},oauth_signature_method={},oauth_timestamp={},oauth_token={},oauth_version={}",
                consumer_key, oauth_nonce, encoded_signature, oauth_signature_method, oauth_timestamp, oauth_token, &oauth_version)
            );
        if since.is_some() && until.is_some() {
            signed_tweet_count_request = signed_tweet_count_request
                .query("end_time", until.as_ref().unwrap())
                .query("start_time", since.as_ref().unwrap());
        } else if since.is_some() {
            signed_tweet_count_request =
                signed_tweet_count_request.query("start_time", since.as_ref().unwrap());
        } else if until.is_some() {
            signed_tweet_count_request =
                signed_tweet_count_request.query("end_time", until.as_ref().unwrap());
        }

        let signed_tweet_count_response = signed_tweet_count_request.call();

        let signed_tweet_count_response = match signed_tweet_count_response {
            Ok(res) => res,
            Err(e) => {
                panic!("{}", e);
            }
        };
        let response_object: ResponseObject<TweetCount> =
            serde_json::from_reader(signed_tweet_count_response.into_reader()).unwrap();

        response_object.data.meta.total_tweet_count
    }

    fn fetch_timeline(
        &self,
        mut since: Option<String>,
        mut until: Option<String>,
    ) -> Result<Vec<Tweet>> {
        info!("Pull the target tweets");
        if self.work_path.exists() {
            debug!("Work file {} will be overwritten", self.work_path.display());
        } else {
            debug!("Work file {} will be created", self.work_path.display());
        }

        if since.is_some() {
            let mut since_date = String::new();
            since_date.push_str(since.as_ref().unwrap());
            since_date.push_str("T00:00:00Z");
            since = Some(since_date);
        }
        if until.is_some() {
            let mut until_date = String::new();
            until_date.push_str(until.as_ref().unwrap());
            until_date.push_str("T00:00:00Z");
            until = Some(until_date);
        }

        let oauth_nonce = Uuid::new_v4();
        let oauth_token = &self.user_cred.as_ref().unwrap().oauth_token;
        let oauth_token_secret = &self.user_cred.as_ref().unwrap().oauth_token_secret;
        let consumer_key = &self.app_cred.consumer_key;
        let consumer_secret = &self.app_cred.consumer_secret;

        let encoded_consumer_secret: String =
            url::form_urlencoded::byte_serialize(&consumer_secret.as_bytes()).collect();
        let encoded_oauth_token_secret: String =
            url::form_urlencoded::byte_serialize(&oauth_token_secret.as_bytes()).collect();
        let signagure_key = format!("{}&{}", encoded_consumer_secret, encoded_oauth_token_secret);

        // メソッドとURL以外のSignature Data構成要素特定
        let oauth_signature_method = "HMAC-SHA1";
        let oauth_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let oauth_version = "1.0";

        let fetch_timeline_request = self.server.join(&format!(
            "2/users/{}/tweets",
            self.user_cred.as_ref().unwrap().id
        ))?;
        // TODO: つけるquery paramによってsignature data内で配置すべき位置が異なる、これを自動で実現可能な形にするべきだが現状はできていない
        // TODO: この冗長な記載はリクエストを投げる箇所でも同様になっている
        let signature_data: String;
        if since.is_some() && until.is_some() {
            let encoded_until: String =
                url::form_urlencoded::byte_serialize(&until.as_ref().unwrap().as_bytes()).collect();
            let encoded_since: String =
                url::form_urlencoded::byte_serialize(&since.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "end_time={}&max_results=100&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&start_time={}&tweet.fields=created_at%2Cpublic_metrics%2Cattachments",
                &encoded_until,
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version,
                &encoded_since
            );
        } else if since.is_some() {
            let encoded_since: String =
                url::form_urlencoded::byte_serialize(&since.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "max_results=100&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&start_time={}&tweet.fields=created_at%2Cpublic_metrics%2Cattachments",
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version,
                &encoded_since
            );
        } else if until.is_some() {
            let encoded_until: String =
                url::form_urlencoded::byte_serialize(&until.as_ref().unwrap().as_bytes()).collect();
            signature_data = format!(
                "end_time={}&max_results=100&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&tweet.fields=created_at%2Cpublic_metrics%2Cattachments",
                &encoded_until,
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version
            );
        } else {
            signature_data = format!(
                "max_results=100&oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}&tweet.fields=created_at%2Cpublic_metrics%2Cattachments",
                &consumer_key,
                &oauth_nonce,
                &oauth_signature_method,
                &oauth_timestamp,
                &oauth_token,
                &oauth_version
            );
        }

        // https://rust-lang-nursery.github.io/rust-cookbook/encoding/strings.html#percent-encode-a-string
        let request_method = String::from("GET");
        let encoded_request_target: String =
            url::form_urlencoded::byte_serialize(&fetch_timeline_request.as_str().as_bytes())
                .collect();
        let encoded_sigature_data: String =
            url::form_urlencoded::byte_serialize(&signature_data.as_bytes()).collect();
        let joined_signature_data = format!(
            "{}&{}&{}",
            request_method, encoded_request_target, encoded_sigature_data
        );
        debug!("sig data: {}", joined_signature_data);

        debug!("sig key: {}", &signagure_key);
        let hmac_digest =
            hmacsha1::hmac_sha1(&signagure_key.as_bytes(), &joined_signature_data.as_bytes());
        let signature = base64::encode(hmac_digest);
        let encoded_signature: String =
            url::form_urlencoded::byte_serialize(&signature.as_str().as_bytes()).collect();
        debug!("sig base64: {}", signature);

        let mut signed_fetch_timeline_request = self
            .agent
            .request_url(request_method.as_str(), &fetch_timeline_request)
            .query("max_results", "100")
            .query("tweet.fields", "created_at,public_metrics,attachments")
            .set("Authorization", &format!(
                "OAuth oauth_consumer_key={},oauth_nonce={},oauth_signature={},oauth_signature_method={},oauth_timestamp={},oauth_token={},oauth_version={}",
                consumer_key, oauth_nonce, encoded_signature, oauth_signature_method, oauth_timestamp, oauth_token, &oauth_version)
            );
        if since.is_some() && until.is_some() {
            signed_fetch_timeline_request = signed_fetch_timeline_request
                .query("end_time", until.as_ref().unwrap())
                .query("start_time", since.as_ref().unwrap());
        } else if since.is_some() {
            signed_fetch_timeline_request =
                signed_fetch_timeline_request.query("start_time", since.as_ref().unwrap());
        } else if until.is_some() {
            signed_fetch_timeline_request =
                signed_fetch_timeline_request.query("end_time", until.as_ref().unwrap());
        }

        let signed_fetch_timeline_response = signed_fetch_timeline_request.call();

        let signed_fetch_timeline_response = match signed_fetch_timeline_response {
            Ok(res) => res,
            Err(e) => {
                panic!("{}", e);
            }
        };
        let mut file = File::create(&self.work_path)?;
        // load on the object for removing unnecessary prop
        let response_object: ResponseObject<Vec<Tweet>> =
            serde_json::from_reader(signed_fetch_timeline_response.into_reader())?;
        serde_json::to_writer(&mut file, &response_object.data)?;

        debug!("{}", &response_object.data.len());
        for each in &response_object.data {
            debug!("{}", each.id);
        }
        Ok(response_object.data)
    }

    fn delete_liked(&self, _tweet_id_str: &str) -> bool {
        unimplemented!();
    }

    fn delete_tweet(&self, tweet_id_str: &str) -> bool {
        let oauth_nonce = Uuid::new_v4();
        let oauth_token = &self.user_cred.as_ref().unwrap().oauth_token;
        let oauth_token_secret = &self.user_cred.as_ref().unwrap().oauth_token_secret;
        let consumer_key = &self.app_cred.consumer_key;
        let consumer_secret = &self.app_cred.consumer_secret;

        let encoded_consumer_secret: String =
            url::form_urlencoded::byte_serialize(&consumer_secret.as_bytes()).collect();
        let encoded_oauth_token_secret: String =
            url::form_urlencoded::byte_serialize(&oauth_token_secret.as_bytes()).collect();
        let signagure_key = format!("{}&{}", encoded_consumer_secret, encoded_oauth_token_secret);

        // メソッドとURL以外のSignature Data構成要素特定
        let oauth_signature_method = "HMAC-SHA1";
        let oauth_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let oauth_version = "1.0";

        let delete_tweet_request = self
            .server
            .join(&format!("1.1/statuses/destroy/{}.json", tweet_id_str))
            .unwrap();
        let signature_data = format!(
            "oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method={}&oauth_timestamp={}&oauth_token={}&oauth_version={}",
            &consumer_key,
            &oauth_nonce,
            &oauth_signature_method,
            &oauth_timestamp,
            &oauth_token,
            &oauth_version
        );

        // https://rust-lang-nursery.github.io/rust-cookbook/encoding/strings.html#percent-encode-a-string
        let request_method = String::from("POST");
        let encoded_request_target: String =
            url::form_urlencoded::byte_serialize(&delete_tweet_request.as_str().as_bytes())
                .collect();
        let encoded_sigature_data: String =
            url::form_urlencoded::byte_serialize(&signature_data.as_bytes()).collect();
        let joined_signature_data = format!(
            "{}&{}&{}",
            request_method, encoded_request_target, encoded_sigature_data
        );
        debug!("sig data: {}", joined_signature_data);

        debug!("sig key: {}", &signagure_key);
        let hmac_digest =
            hmacsha1::hmac_sha1(&signagure_key.as_bytes(), &joined_signature_data.as_bytes());
        let signature = base64::encode(hmac_digest);
        let encoded_signature: String =
            url::form_urlencoded::byte_serialize(&signature.as_str().as_bytes()).collect();
        debug!("sig base64: {}", signature);

        let signed_delete_tweet_response = self
            .agent
            .request_url(request_method.as_str(), &delete_tweet_request)
            .set("Authorization", &format!(
                "OAuth oauth_consumer_key={},oauth_nonce={},oauth_signature={},oauth_signature_method={},oauth_timestamp={},oauth_token={},oauth_version={}",
                consumer_key, oauth_nonce, encoded_signature, oauth_signature_method, oauth_timestamp, oauth_token, &oauth_version)
            ).call();

        match signed_delete_tweet_response {
            Ok(_) => return true,
            Err(_) => return false,
        };
    }
}
