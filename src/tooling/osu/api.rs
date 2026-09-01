use reqwest::Client;
use serde::Deserialize;

const OSU_API_URL: &str = "https://osu.ppy.sh/api/v2";
const OSU_OAUTH: &str = "https://osu.ppy.sh/oauth/token";


#[derive(Debug, Deserialize)]
struct OAuthResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct OsuUser {
    pub id: u64,
    pub username: String,
    pub last_visit: Option<String>,
    pub statistics: OsuStatistics,
}

#[derive(Debug, Deserialize)]
pub struct OsuStatistics {
    pub global_rank: Option<u32>,
    pub pp: f64,
}

pub struct OsuClient {
    client: Client,
    access_token: String,
}

impl OsuClient {
    pub async fn new(
        client_id: u32,
        client_secret: &str,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::new();

        let response = client
            .post(OSU_OAUTH)
            .json(&serde_json::json!({
                "client_id": client_id,
                "client_secret": client_secret,
                "grant_type": "client_credentials",
                "scope": "public"
            }))
            .send()
            .await?
            .error_for_status()?
            .json::<OAuthResponse>()
            .await?;

        Ok(Self {
            client,
            access_token: response.access_token,
        })
    }

    pub async fn get_user(
        &self,
        user: &str,
    ) -> Result<OsuUser, reqwest::Error> {
        self.client
            .get(format!("{OSU_API_URL}/users/{user}"))
            .bearer_auth(&self.access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<OsuUser>()
            .await
    }
}