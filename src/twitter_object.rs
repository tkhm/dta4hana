use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ResponseObject<T> {
    pub data: T,
}

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Tweet {
    pub id: String,
    pub created_at: String,
    pub public_metrics: PublicMetrics,
    pub attachments: Option<Attachments>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct TweetCount {
    pub meta: TweetCountMeta,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct TweetCountMeta {
    pub total_tweet_count: u32,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PublicMetrics {
    pub retweet_count: u32,
    pub reply_count: u32,
    pub like_count: u32,
    pub quote_count: u32,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Attachments {
    pub media_keys: Vec<String>,
}
