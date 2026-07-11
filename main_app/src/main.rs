mod entity;

use askama::Template;
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dotenvy::dotenv;
use entity::{accounts, users};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, Database, DatabaseConnection, EntityTrait, ExprTrait,
    QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use url::Url;

const GOOGLE_DOCUMENT_DISCOVERY_URL: &str =
    "https://accounts.google.com/.well-known/openid-configuration";

#[derive(Serialize)]
struct CodeExchangeRequest<'a> {
    code: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    grant_type: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
struct UserProfile {
    name: String,
    email: String,
    sub: String,
    email_verified: bool,
}

#[derive(Deserialize, Serialize)]
struct Tokens {
    access_token: String,
    expires_in: u64,
    id_token: String,
    scope: String,
    token_type: String,
    refresh_token: Option<String>,
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

// State structure must be Clone
#[derive(Clone)]
struct AppState {
    // Arc allows efficient, thread-safe sharing of resources
    client_id: Arc<String>,
    client_secret: Arc<String>,
    redirect_uri: Arc<String>,
    db_pool: DatabaseConnection,
}

// Define your target structure
#[derive(Deserialize)]
struct Code {
    code: String, // Option makes it optional
}
#[tokio::main]
async fn main() {
    // Load environment variables from the .env file
    dotenv().expect(".env file not found");

    // Read the variable using standard library
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID must be set");
    let client_secret = env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redirect_uri =
        "https://gentle-assuring-stingray.ngrok-free.app/auth/callback/google".to_owned();
    let db: DatabaseConnection = Database::connect(database_url).await.unwrap();

    let shared_state = AppState {
        client_id: Arc::new(client_id.clone()),
        client_secret: Arc::new(client_secret.clone()),
        redirect_uri: Arc::new(redirect_uri.clone()),
        db_pool: db,
    };

    let hello = HelloTemplate {
        _client_id: client_id,
        _redirect_uri: redirect_uri,
    }
    .render()
    .unwrap();

    let app = Router::new()
        .route("/", get(|| async { Html(hello) }))
        .route("/google/login", get(handle_google_login))
        .route("/auth/callback/google", get(handle_google_callback))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_google_callback(
    State(state): State<AppState>,
    Query(code): Query<Code>,
) -> impl axum::response::IntoResponse {
    println!("CODE: {}", code.code);
    let authorization_endpoint: TokenEndpoint = reqwest::get(GOOGLE_DOCUMENT_DISCOVERY_URL)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let payload = CodeExchangeRequest {
        client_id: (*state.client_id).clone(),
        client_secret: (*state.client_secret).clone(),
        redirect_uri: (*state.redirect_uri).clone(),
        code: code.code,
        grant_type: "authorization_code",
    };
    println!("DEBUG: {}", authorization_endpoint.token_endpoint.clone());
    let client = reqwest::Client::new();
    let response: Tokens = client
        .post(authorization_endpoint.token_endpoint) // Target endpoint
        .json(&payload) // Serializes struct and sets JSON headers
        .send() // Dispatches request asynchronously
        .await
        .unwrap()
        .json()
        .await
        .unwrap(); // Unwraps execution future

    let decoded_bytes = URL_SAFE_NO_PAD
        .decode(response.id_token.clone().split(".").collect::<Vec<&str>>()[1])
        .unwrap();
    let decoded_str = String::from_utf8(decoded_bytes).unwrap();

    let user_info: UserProfile = serde_json::from_str(decoded_str.as_str()).unwrap();

    match entity::prelude::Accounts::find()
        .filter(
            Condition::all().add(
                accounts::Column::ProviderId
                    .eq(user_info.sub.clone())
                    .and(accounts::Column::Provider.eq("google".to_string())),
            ),
        )
        .one(&state.db_pool)
        .await
        .unwrap()
    {
        Some(_) => "Login successfullly",
        None => {
            let txn = state.db_pool.begin().await.unwrap();

            let user = users::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                name: sea_orm::ActiveValue::Set(user_info.name),
                email: sea_orm::ActiveValue::Set(user_info.email),
                email_verified: sea_orm::ActiveValue::Set(user_info.email_verified),
                created_at: sea_orm::ActiveValue::NotSet,
            }
            .insert(&state.db_pool)
            .await
            .unwrap();

            let account: accounts::Model = accounts::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                user_id: sea_orm::ActiveValue::Set(user.id),
                provider: sea_orm::ActiveValue::Set("google".to_string()),
                password: sea_orm::ActiveValue::Set(None),
                provider_id: sea_orm::ActiveValue::Set(Some(user_info.sub)),
                created_at: sea_orm::ActiveValue::NotSet,
            }
            .insert(&state.db_pool)
            .await
            .unwrap();

            txn.commit().await.unwrap();

            println!("User: {:?}\nAccount: {:?}", user, account);

            "Registration completed successfully"
        }
    }
}

async fn handle_google_login(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();

    url.query_pairs_mut()
        .append_pair("client_id", &state.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &state.redirect_uri)
        .append_pair("scope", "openid profile email");

    axum::response::Redirect::temporary(url.as_str())
}
