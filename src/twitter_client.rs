//! Twitter API Client
//! It calls APIs and has its required implementation(e.g. handling OAuth flow)
//! Define it as trait and implement it for the testability(using mock)
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::env;
use url::Url;
use uuid::Uuid;

use crate::twitter_object::{ResponseObject, Tweet, User};

/// Twitter Client
/// It needs to know the endpoints and all required credentials
pub struct TwitterClient {
    agent: ureq::Agent,
    server: Url,
    app_cred: TwitterAppCredential,
    user_cred: Option<TwitterAppUserCredential>,
}
/// App side credentials
/// It will be passed in build time and it will not be changed by app users
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterAppCredential {
    pub api_key: String,
    pub consumer_key: String,
    pub consumer_secret: String,
}
/// User side credentials
/// It will be updated after login
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterAppUserCredential {
    pub username: String,
    pub id: String,
    pub oauth_token: String,
    pub oauth_token_secret: String,
}

#[cfg(test)]
use mockall::{automock, predicate::*};
#[cfg_attr(test, automock)]
pub trait TwitterClientTrait {
    fn new(
        api_key: String,
        consumer_key: String,
        consumer_secret: String,
        user_cred: Option<TwitterAppUserCredential>,
    ) -> Self;
    fn delete_liked(&self, tweet_id_str: &str) -> Result<()>;
    fn delete_tweet(&self, tweet_id_str: &str) -> Result<()>;
    fn fetch_timeline(&self, since: Option<String>, until: Option<String>) -> Result<Vec<Tweet>>;
    fn init_user_cred(self, user_cred: TwitterAppUserCredential) -> Result<TwitterClient>;
    fn login(&self) -> Result<TwitterAppUserCredential>;
}

impl TwitterClientTrait for TwitterClient {
    /// Constructs new Twitter Client
    /// * api_key: Bearer Token
    /// * consumer_key: Consumer Key
    /// * consumer_secret: Consumer Secret
    /// * user_cred: It is optional, because this client is also needed in the first time use(i.e. login),
    ///   but you can't call all other apis until you put this credential.
    ///   You can fill this later with [`TwitterClient::init_user_cred()`]
    fn new(
        api_key: String,
        consumer_key: String,
        consumer_secret: String,
        user_cred: Option<TwitterAppUserCredential>,
    ) -> Self {
        let server = match Url::parse("https://api.twitter.com") {
            Ok(url) => url,
            Err(_) => panic!("API Endpoints is not valid."),
        };
        let agent: ureq::Agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(5))
            .timeout_write(Duration::from_secs(5))
            .build();

        let app_cred = TwitterAppCredential {
            api_key,
            consumer_key,
            consumer_secret,
        };

        TwitterClient {
            agent,
            server,
            app_cred,
            user_cred,
        }
    }

    /// Delete(unliked) your liked tweet from your liked tweets
    /// * _tweet_id_str: target tweet id
    fn delete_liked(&self, _tweet_id_str: &str) -> Result<()> {
        unimplemented!();
    }

    /// Delete your liked tweet
    /// * tweet_id_str: target tweet id
    fn delete_tweet(&self, tweet_id_str: &str) -> Result<()> {
        let user_cred = match &self.user_cred {
            Some(cred) => cred,
            None => return Err(anyhow::anyhow!("Credential is not loaded.")),
        };

        let oauth_nonce = Uuid::new_v4();
        let oauth_token = &user_cred.oauth_token;
        let oauth_token_secret = &user_cred.oauth_token_secret;
        let consumer_key = &self.app_cred.consumer_key;
        let consumer_secret = &self.app_cred.consumer_secret;

        let encoded_consumer_secret: String =
            url::form_urlencoded::byte_serialize(consumer_secret.as_bytes()).collect();
        let encoded_oauth_token_secret: String =
            url::form_urlencoded::byte_serialize(oauth_token_secret.as_bytes()).collect();
        let signagure_key = format!("{}&{}", encoded_consumer_secret, encoded_oauth_token_secret);

        // メソッドとURL以外のSignature Data構成要素特定
        let oauth_signature_method = "HMAC-SHA1";
        let oauth_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let oauth_version = "1.0";

        let delete_tweet_request = self
            .server
            .join(&format!("1.1/statuses/destroy/{}.json", tweet_id_str))?;
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
            url::form_urlencoded::byte_serialize(delete_tweet_request.as_str().as_bytes())
                .collect();
        let encoded_sigature_data: String =
            url::form_urlencoded::byte_serialize(signature_data.as_bytes()).collect();
        let joined_signature_data = format!(
            "{}&{}&{}",
            request_method, encoded_request_target, encoded_sigature_data
        );
        let hmac_digest =
            hmacsha1::hmac_sha1(signagure_key.as_bytes(), joined_signature_data.as_bytes());
        let signature = base64::encode(hmac_digest);
        let encoded_signature: String =
            url::form_urlencoded::byte_serialize(signature.as_str().as_bytes()).collect();

        let signed_delete_tweet_response = self
            .agent
            .request_url(request_method.as_str(), &delete_tweet_request)
            .set("Authorization", &format!(
                "OAuth oauth_consumer_key={},oauth_nonce={},oauth_signature={},oauth_signature_method={},oauth_timestamp={},oauth_token={},oauth_version={}",
                consumer_key, oauth_nonce, encoded_signature, oauth_signature_method, oauth_timestamp, oauth_token, &oauth_version)
            ).call();

        match signed_delete_tweet_response {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Failed to delete.")),
        }
    }

    /// Retrieve the tweets
    /// It will get 100 tweets(MAX and fixed value)
    /// * since: the first date of getting tweets e.g. 2022-01-01
    ///   It will be attached time and timezone after that date like 2022-01-01T00:00:00Z
    /// * until: the last date of getting tweets e.g. 2022-12-31
    ///   It will be attached time and timezone after that date like 2022-12-31T00:00:00Z
    fn fetch_timeline(
        &self,
        since_arg: Option<String>,
        until_arg: Option<String>,
    ) -> Result<Vec<Tweet>> {
        let user_cred = match &self.user_cred {
            Some(cred) => cred,
            None => return Err(anyhow::anyhow!("Credential is not loaded.")),
        };

        info!("Pull the target tweets");
        let since = match since_arg {
            Some(since_arg) => {
                let mut since_date = String::new();
                since_date.push_str(&since_arg);
                since_date.push_str("T00:00:00Z");
                Some(since_date)
            }
            None => None,
        };
        let until = match until_arg {
            Some(until_arg) => {
                let mut until_date = String::new();
                until_date.push_str(&until_arg);
                until_date.push_str("T00:00:00Z");
                Some(until_date)
            }
            None => None,
        };

        let oauth_nonce = Uuid::new_v4();
        let oauth_token = &user_cred.oauth_token;
        let oauth_token_secret = &user_cred.oauth_token_secret;
        let consumer_key = &self.app_cred.consumer_key;
        let consumer_secret = &self.app_cred.consumer_secret;

        let encoded_consumer_secret: String =
            url::form_urlencoded::byte_serialize(consumer_secret.as_bytes()).collect();
        let encoded_oauth_token_secret: String =
            url::form_urlencoded::byte_serialize(oauth_token_secret.as_bytes()).collect();
        let signagure_key = format!("{}&{}", encoded_consumer_secret, encoded_oauth_token_secret);

        // メソッドとURL以外のSignature Data構成要素特定
        let oauth_signature_method = "HMAC-SHA1";
        let oauth_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let oauth_version = "1.0";

        let fetch_timeline_request = self
            .server
            .join(&format!("2/users/{}/tweets", &user_cred.id))?;
        // TODO: つけるquery paramによってsignature data内で配置すべき位置が異なる、これを自動で実現可能な形にするべきだが現状はできていない
        // TODO: この冗長な記載はリクエストを投げる箇所でも同様になっている
        let signature_data: String;
        if since.is_some() && until.is_some() {
            let encoded_until: String =
                url::form_urlencoded::byte_serialize(until.as_ref().unwrap().as_bytes()).collect();
            let encoded_since: String =
                url::form_urlencoded::byte_serialize(since.as_ref().unwrap().as_bytes()).collect();
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
                url::form_urlencoded::byte_serialize(since.as_ref().unwrap().as_bytes()).collect();
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
                url::form_urlencoded::byte_serialize(until.as_ref().unwrap().as_bytes()).collect();
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
            url::form_urlencoded::byte_serialize(fetch_timeline_request.as_str().as_bytes())
                .collect();
        let encoded_sigature_data: String =
            url::form_urlencoded::byte_serialize(signature_data.as_bytes()).collect();
        let joined_signature_data = format!(
            "{}&{}&{}",
            request_method, encoded_request_target, encoded_sigature_data
        );

        let hmac_digest =
            hmacsha1::hmac_sha1(signagure_key.as_bytes(), joined_signature_data.as_bytes());
        let signature = base64::encode(hmac_digest);
        let encoded_signature: String =
            url::form_urlencoded::byte_serialize(signature.as_str().as_bytes()).collect();

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
        // load on the object for removing unnecessary prop
        let response_object: ResponseObject<Vec<Tweet>> =
            serde_json::from_reader(signed_fetch_timeline_response.into_reader())?;

        debug!("Got: {} tweets", &response_object.data.len());
        Ok(response_object.data)
    }

    /// * user_cred: app defined user credential struct
    ///   It is expected to come from [`TwitterClient::login()`]
    fn init_user_cred(mut self, user_cred: TwitterAppUserCredential) -> Result<TwitterClient> {
        self.user_cred.replace(user_cred);
        Ok(self)
    }

    /// Login and return the user credentials(oauth_token and oauth_token_secret)
    /// It is based on PIN-based authorization and it requires to login on your browser and type the PIN
    /// ref: <https://developer.twitter.com/ja/docs/basics/authentication/overview/pin-based-oauth>
    fn login(&self) -> Result<TwitterAppUserCredential> {
        // 情報がないのでログイン処理
        // ユーザーからの入力
        info!("Please input your Twitter username:");
        let mut username = String::new();
        std::io::stdin().read_line(&mut username)?;

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
        let req_oauth_token = match request_token_keys.get("oauth_token") {
            Some(value) => value.to_string(),
            None => return Err(anyhow::anyhow!("No token is found")),
        };

        // 認証
        let authorize_request = self
            .server
            .join(&format!("oauth/authorize?oauth_token={}", req_oauth_token))?;

        info!(
            "Please open this URL in your browser: {}",
            authorize_request.to_string()
        );

        // ユーザーからの入力
        info!("After authorize app, please input PIN number on the screen for complete the authorization process:");
        let mut s = String::new();
        std::io::stdin().read_line(&mut s)?;

        // 認証完了
        let access_token_request = self.server.join(&format!(
            "oauth/access_token?oauth_token={}&oauth_verifier={}",
            req_oauth_token,
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

        // note: this oauth_token and request's oauth_token is not the same
        let oauth_token = match access_token_keys.get("oauth_token") {
            Some(value) => value.to_string(),
            None => return Err(anyhow::anyhow!("No token is found")),
        };
        let oauth_token_secret = match access_token_keys.get("oauth_token_secret") {
            Some(value) => value.to_string(),
            None => return Err(anyhow::anyhow!("No token secret is found")),
        };
        let user_cred = TwitterAppUserCredential {
            username,
            id: user_id,
            oauth_token,
            oauth_token_secret,
        };
        Ok(user_cred)
    }
}
