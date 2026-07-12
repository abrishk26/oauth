use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

#[derive(Serialize)]
struct CodeExchangeRequest<'a> {
    code: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    grant_type: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    // AccessToken is the token that authorizes and authenticates
    // the requests.
    pub access_token: String,

    // TokenType is the type of token.
    // The Type method returns either this or "Bearer", the default.
    pub token_type: String,

    // RefreshToken is a token that's used by the application
    // (as opposed to the user) to refresh the access token
    // if it expires.
    pub refresh_token: Option<String>,

    // specifies how many seconds later the token expires,
    // relative to an unknown time base approximately around "now".
    pub expires_in: u64,

    // id_token is a jwt token that contains users information in the claims without making further request
    pub id_token: String,
}
#[derive(Clone)]
pub struct Endpoint {
    pub auth_url: String,
    pub token_url: String,
}

#[derive(Clone)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub endpoint: Endpoint,
}

#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.message.as_str();
        write!(f, "unable to read configuration at {path}")
    }
}

impl Config {
    pub fn auth_code_url(&self, state: &str) -> Result<String, ConfigError> {
        let mut url = Url::parse(&&self.endpoint.auth_url.as_str()).map_err(|e| ConfigError {
            message: e.to_string(),
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code");

        if self.redirect_uri.len() > 0 {
            url.query_pairs_mut()
                .append_pair("redirect_uri", &self.redirect_uri);
        }

        if self.scopes.len() > 0 {
            url.query_pairs_mut()
                .append_pair("scope", &(self.scopes).join(" "));
        }

        if state.len() > 0 {
            url.query_pairs_mut().append_pair("state", state);
        }

        Ok(url.to_string())
    }

    pub async fn exchange(&self, code: &str) -> Result<Token, ConfigError> {
        let payload = CodeExchangeRequest {
            client_id: (self.client_id).clone(),
            client_secret: (self.client_secret).clone(),
            redirect_uri: (self.redirect_uri).clone(),
            code: code.to_string(),
            grant_type: "authorization_code",
        };

        let client = reqwest::Client::new();
        Ok(client
            .post(self.endpoint.token_url.clone()) // Target endpoint
            .json(&payload) // Serializes struct and sets JSON headers
            .send() // Dispatches request asynchronously
            .await
            .map_err(|e| ConfigError {
                message: e.to_string(),
            })?
            .json::<Token>()
            .await
            .map_err(|e| ConfigError {
                message: e.to_string(),
            })?)
    }
}
