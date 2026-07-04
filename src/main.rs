use axum::{Router, response::{Html, Json}, routing::{get}, extract::{State, Query}};
use url::Url;
use dotenvy::dotenv;
// use rand::distr::{Alphanumeric, SampleString};
use askama::Template;
use std::{collections::HashMap, env}; // bring trait in scope

const GOOGLE_DOCUMENT_DISCOVERY_URL: &str = "https://accounts.google.com/.well-known/openid-configuration";

#[derive(Serialize)]
struct CodeExchangeRequest<'a> {
    code: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    grant_type: &'a str,
}

#[derive(Deserialize, Debug)]
struct TokenEndpoint {
    token_endpoint: String,
}

#[derive(Template)] // this will generate the code...
#[template(path = "hello.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct HelloTemplate {
    // the name of the struct can be anything
    _client_id: String, // the field name should match the variable name
    _redirect_uri: String, // the field name should match the variable name
                       // in your template
}

use std::sync::Arc;

// State structure must be Clone
#[derive(Clone)]
struct AppState {
    // Arc allows efficient, thread-safe sharing of resources
    client_id: Arc<String>,
    client_secret: Arc<String>,
    redirect_uri: Arc<String>,
}

use serde::{Deserialize, Serialize};
// Define your target structure
#[derive(Deserialize)]
struct Code {
    code: String,      // Option makes it optional
}
#[tokio::main]
async fn main() {
    // Load environment variables from the .env file
    dotenv().expect(".env file not found");

    // Read the variable using standard library
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID must be set");
    let client_secret = env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set");
    // let response_type = "code".to_owned();
    let redirect_uri =
        "https://gentle-assuring-stingray.ngrok-free.app/auth/callback/google".to_owned();
    // let scope = "openid email".to_owned();
    let shared_state = AppState {
        client_id: Arc::new(client_id.clone()),
        client_secret: Arc::new(client_secret.clone()),
        redirect_uri: Arc::new(redirect_uri.clone()),
    };

    let hello = HelloTemplate {
        _client_id: client_id,
        _redirect_uri: redirect_uri,
    }
    .render()
    .unwrap(); // instantiate your struct

    let app = Router::new()
        .route("/", get(|| async { Html(hello) }))
        .route(
            "/google/login",
            get(handle_google_login),
        )
        .route("/auth/callback/google", get(handle_google_callback))
        .with_state(shared_state)
    ;

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    // println!("{:?}", response.text().await.unwrap());
}

async fn handle_google_callback(State(state): State<AppState>, Query(code): Query<Code>) -> impl axum::response::IntoResponse {
    println!("CODE: {}", code.code);
    let authorization_endpoint: TokenEndpoint = reqwest::get(GOOGLE_DOCUMENT_DISCOVERY_URL).await.unwrap().json().await.unwrap();

    let payload = CodeExchangeRequest {
        client_id: (*state.client_id).clone(),
        client_secret: (*state.client_secret).clone(),
        redirect_uri: (*state.redirect_uri).clone(),
        code: code.code,
        grant_type: "authorization_code"
    };
    println!("DEBUG: {}", authorization_endpoint.token_endpoint.clone());
    let client = reqwest::Client::new();
    let response: HashMap<String, serde_json::Value> = client.post(authorization_endpoint.token_endpoint) // Target endpoint
            .form(&payload)                  // Serializes struct and sets JSON headers
            .send()                          // Dispatches request asynchronously
            .await.unwrap().json().await.unwrap();                         // Unwraps execution future

   Json(response)
}

async fn handle_google_login(
    State(state): State<AppState>
) -> impl axum::response::IntoResponse {
    // let response = reqwest::Client::new().get("https://accounts.google.com/o/oauth2/v2/auth").query(
    //     &[("client_id", client_id), ("response_type", "code".to_string()), ("redirect_uri", redirect_uri), ("scope", "openid email".to_string())]
    // ).send().await.expect("error while sending request");
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
        .unwrap();

    url.query_pairs_mut()
        .append_pair("client_id", &state.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &state.redirect_uri)
        .append_pair("scope", "openid email");

    axum::response::Redirect::temporary(url.as_str())
}
