use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
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

    // extra optionally contains extra metadata from the server
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl Token {
    // type returns self.token_type if non-empty, else "Bearer".
    pub fn token_type(&self) -> &str {
        return match self.token_type.to_ascii_lowercase().as_str() {
            "bearer" => "Bearer",
            "mac" => "MAC",
            "basic" => "Basic",
            tt if tt.len() > 0 => self.token_type.as_str(),
            _ => "Bearer",
        };
    }
    
    // Extra returns an extra field.
    // Extra fields are key-value pairs returned by the server as
    // part of the token retrieval response.
    pub fn get_extra(&self, key: &str) -> Option<&serde_json::Value> {
       self.extra.get(key) 
    }
}

#[derive(Clone)]
pub struct Endpoint {
    pub auth_endpoint: String,
    pub token_endpoint: String,
}

#[derive(Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub endpoint: Endpoint,
}

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = self.message.as_str();
        write!(f, "{msg}")
    }
}

impl OAuthClient {
    pub fn auth_code_url(&self, state: &str) -> Result<String, Error> {
        let mut url = Url::parse(self.endpoint.auth_endpoint.as_str()).map_err(|e| Error {
            message: e.to_string(),
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &(self.scopes).join(" "))
            .append_pair("state", state);

        Ok(url.to_string())
    }

    pub async fn exchange(&self, code: &str) -> Result<Token, Error> {
        let payload = CodeExchangeRequest {
            client_id: (self.client_id).clone(),
            client_secret: (self.client_secret).clone(),
            redirect_uri: (self.redirect_uri).clone(),
            code: code.to_string(),
            grant_type: "authorization_code",
        };

        let client = reqwest::Client::new();
        Ok(client
            .post(self.endpoint.token_endpoint.clone()) // Target endpoint
            .json(&payload) // Serializes struct and sets JSON headers
            .send() // Dispatches request asynchronously
            .await
            .map_err(|e| Error {
                message: e.to_string(),
            })?
            .json::<Token>()
            .await
            .map_err(|e| Error {
                message: e.to_string(),
            })?)
    }
}
