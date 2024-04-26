use axum::{
    body::Body,
    extract::{Query, State},
    http::{
        header::{self, AUTHORIZATION},
        Request, Response, StatusCode, Uri,
    },
    middleware::Next,
    response::{IntoResponse, Redirect},
    Extension, Json,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use minijinja::context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{locales::Lang, RhombusRouterState};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub discord_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: i64,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub discord_id: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClientUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub discord_id: String,
}

pub type MaybeClientUser = Option<ClientUser>;

#[derive(Debug, Serialize, Clone)]
pub struct ErrorResponse {
    pub message: String,
}

pub async fn enforce_auth_middleware(
    Extension(user): Extension<MaybeClientUser>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    if user.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "Unauthorized".to_string(),
            }),
        ));
    }
    let user = user.unwrap();

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

pub async fn auth_injector_middleware(
    cookie_jar: CookieJar,
    State(data): State<RhombusRouterState>,
    mut req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let token = cookie_jar
        .get("rhombus-token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(&AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| {
                    auth_value
                        .strip_prefix("Bearer ")
                        .map(|bearer| bearer.to_owned())
                })
        });

    if token.is_none() {
        let client_user: MaybeClientUser = None;
        req.extensions_mut().insert(client_user);
        return next.run(req).await;
    }
    let token = token.unwrap();

    let claims = decode::<TokenClaims>(
        &token,
        &DecodingKey::from_secret(data.settings.jwt_secret.as_ref()),
        &Validation::default(),
    );
    if claims.is_err() {
        let client_user: MaybeClientUser = None;
        req.extensions_mut().insert(client_user);
        return next.run(req).await;
    }
    let claims = claims.unwrap().claims;

    let client_user: MaybeClientUser = Some(ClientUser {
        id: claims.sub,
        name: claims.name,
        email: claims.email,
        avatar: claims.avatar,
        discord_id: claims.discord_id,
    });
    req.extensions_mut().insert(client_user);
    next.run(req).await
}

#[derive(Deserialize)]
pub struct SignInParams {
    token: Option<String>,
}

pub async fn route_signin(
    state: State<RhombusRouterState>,
    Extension(user): Extension<MaybeClientUser>,
    Extension(lang): Extension<Lang>,
    uri: Uri,
    params: Query<SignInParams>,
) -> Response<Body> {
    let (invite_token_cookie, team_name) = if let Some(url_invite_token) = &params.token {
        let team = state
            .db
            .get_team_from_invite_token(url_invite_token)
            .await
            .unwrap_or(None);

        if let (Some(team), Some(user)) = (&team, &user) {
            state.db.add_user_to_team(user.id, team.id).await.unwrap();
            return Redirect::to("/team").into_response();
        }

        (
            Cookie::build(("rhombus-invite-token", url_invite_token))
                .path("/")
                .max_age(time::Duration::hours(1))
                .same_site(SameSite::Lax)
                .http_only(true),
            team.map(|t| t.name),
        )
    } else {
        (
            Cookie::build(("rhombus-invite-token", ""))
                .path("/")
                .max_age(time::Duration::hours(-1))
                .same_site(SameSite::Lax)
                .http_only(true),
            None,
        )
    };

    let discord_signin_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}/signin/discord&response_type=code&scope=identify+guilds.join",
        state.settings.discord.client_id,
        state.settings.location_url,
    );

    let html = state
        .jinja
        .get_template("signin.html")
        .unwrap()
        .render(context! {
            lang => lang,
            user => user,
            uri => uri.to_string(),
            location_url => state.settings.location_url,
            discord_signin_url => discord_signin_url,
            og_image => format!("{}/og-image.png", state.settings.location_url),
            team_name => team_name
        })
        .unwrap();

    Response::builder()
        .header("content-type", "text/html")
        .header("set-cookie", invite_token_cookie.to_string())
        .body(html.into())
        .unwrap()
}

#[derive(Deserialize)]
pub struct DiscordCallback {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DiscordOAuthToken {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: String,
    scope: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct DiscordProfile {
    id: String,
    email: String,
    username: String,
    avatar: Option<String>,
    global_name: String,
    discriminator: Option<String>,
}

pub async fn route_discord_callback(
    state: State<RhombusRouterState>,
    params: Query<DiscordCallback>,
    cookie_jar: CookieJar,
) -> impl IntoResponse {
    if let Some(error) = &params.error {
        tracing::error!("Discord returned an error: {}", error);
        let json_error = ErrorResponse {
            message: format!("Discord returned an error: {}", error),
        };
        return (StatusCode::BAD_REQUEST, Json(json_error)).into_response();
    }

    let code = if let Some(code) = &params.code {
        code
    } else {
        let json_error = ErrorResponse {
            message: "Discord did not return a code".to_string(),
        };
        return (StatusCode::BAD_REQUEST, Json(json_error)).into_response();
    };

    let client = Client::new();
    let res = client
        .post("https://discord.com/api/oauth2/token")
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .basic_auth(
            &state.settings.discord.client_id,
            Some(&state.settings.discord.client_secret),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            (
                "redirect_uri",
                &format!("{}/signin/discord", state.settings.location_url),
            ),
        ])
        .send()
        .await
        .unwrap();

    if !res.status().is_success() {
        let json_error = ErrorResponse {
            message: format!("Discord returned an error: {:?}", res.text().await),
        };
        return (StatusCode::BAD_REQUEST, Json(json_error)).into_response();
    }

    let oauth_token = res.json::<DiscordOAuthToken>().await.unwrap();

    let res = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&oauth_token.access_token)
        .send()
        .await
        .unwrap();
    if !res.status().is_success() {
        let json_error = ErrorResponse {
            message: format!("Discord returned an error: {:?}", res.text().await),
        };
        return (StatusCode::BAD_REQUEST, Json(json_error)).into_response();
    }

    let profile = res.json::<DiscordProfile>().await.unwrap();

    // join the user to the guild
    let client = Client::new();
    let res = client
        .put(format!(
            "https://discord.com/api/guilds/{}/members/{}",
            state.settings.discord.guild_id, profile.id
        ))
        .header(
            "Authorization",
            format!("Bot {}", state.settings.discord.bot_token),
        )
        .json(&json!({
            "access_token": oauth_token.access_token,
        }))
        .send()
        .await
        .unwrap();
    if !res.status().is_success() {
        let json_error = ErrorResponse {
            message: format!("Discord returned an error: {:?}", res.text().await),
        };
        return (StatusCode::BAD_REQUEST, Json(json_error)).into_response();
    }

    // get the user's avatar
    let avatar = if let Some(avatar) = profile.avatar {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}",
            profile.id,
            avatar,
            if avatar.starts_with("a_") {
                "gif"
            } else {
                "png"
            }
        )
    } else {
        let default_avatar_number = profile.discriminator.unwrap().parse::<i64>().unwrap() % 5;
        format!(
            "https://cdn.discordapp.com/embed/avatars/{}.png",
            default_avatar_number
        )
    };

    let (user_id, _) = state
        .db
        .upsert_user(&profile.global_name, &profile.email, &avatar, &profile.id)
        .await;

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::try_minutes(60).unwrap()).timestamp() as usize;
    let claims = TokenClaims {
        sub: user_id,
        name: profile.global_name,
        email: profile.email,
        avatar,
        discord_id: profile.id,
        exp,
        iat,
    };

    let mut response = Redirect::permanent("/team").into_response();
    let headers = response.headers_mut();

    if let Some(cookie_invite_token) = cookie_jar.get("rhombus-invite-token").map(|c| c.value()) {
        if let Some(team) = state
            .db
            .get_team_from_invite_token(cookie_invite_token)
            .await
            .unwrap_or(None)
        {
            _ = state.db.add_user_to_team(user_id, team.id).await;
        }

        let delete_cookie = Cookie::build(("rhombus-invite-token", ""))
            .path("/")
            .max_age(time::Duration::hours(-1))
            .same_site(SameSite::Lax)
            .http_only(true);

        headers.append(
            header::SET_COOKIE,
            delete_cookie.to_string().parse().unwrap(),
        );
    }

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.settings.jwt_secret.as_ref()),
    )
    .unwrap();

    let cookie = Cookie::build(("rhombus-token", token.to_owned()))
        .path("/")
        .max_age(time::Duration::hours(1))
        .same_site(SameSite::Lax)
        .http_only(true);

    headers.append(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    response
}

pub async fn route_signout() -> impl IntoResponse {
    let cookie = Cookie::build(("rhombus-token", ""))
        .path("/")
        .max_age(time::Duration::hours(-1))
        .same_site(SameSite::Lax)
        .http_only(true);

    let mut response = Redirect::to("/signin").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    response
}
