//! Twitter API response object definition
use serde::{Deserialize, Serialize};

/// Wrapper of the response
/// `T` is depending on the endpoints, but always it will be wrapped with `data`
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

/// Will be used for chekcing how many likes, retweets and replies on the tweet
#[derive(Deserialize, Debug, Serialize)]
pub struct PublicMetrics {
    pub retweet_count: u32,
    pub reply_count: u32,
    pub like_count: u32,
    pub quote_count: u32,
}

/// Will be used for chekcing the attachments
#[derive(Deserialize, Debug, Serialize)]
pub struct Attachments {
    pub media_keys: Vec<String>,
}
