use axum::{
    extract::State,
    http::Uri,
    response::{Html, IntoResponse},
    Extension, Form,
};
use minijinja::context;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use reqwest::StatusCode;
use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

use super::{auth::User, locales::Languages, router::RouterState};

pub fn create_team_invite_token() -> String {
    Alphanumeric.sample_string(&mut thread_rng(), 16)
}

pub async fn route_team(
    state: State<RouterState>,
    Extension(user): Extension<User>,
    Extension(lang): Extension<Languages>,
    uri: Uri,
) -> impl IntoResponse {
    let team = state.db.get_team_from_id(user.team_id).await.unwrap();

    let team_invite_url = format!(
        "{}/signin?token={}",
        state.settings.location_url, team.invite_token
    );

    Html(
        state
            .jinja
            .get_template("team.html")
            .unwrap()
            .render(context! {
                lang => lang,
                user => user,
                team => team,
                team_invite_url => team_invite_url,
                uri => uri.to_string(),
                location_url => state.settings.location_url,
                og_image => format!("{}/og-image.png", state.settings.location_url),
            })
            .unwrap(),
    )
}

pub async fn route_team_roll_token(
    state: State<RouterState>,
    Extension(user): Extension<User>,
    Extension(lang): Extension<Languages>,
) -> Result<impl IntoResponse, StatusCode> {
    if !user.is_team_owner {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let new_invite_token = state.db.roll_invite_token(user.team_id).await.unwrap();

    let team_invite_url = format!(
        "{}/signin?token={}",
        state.settings.location_url, new_invite_token
    );

    Ok(Html(
        state
            .jinja
            .get_template("team-token.html")
            .unwrap()
            .render(context! {
                lang => lang,
                team_invite_url => team_invite_url,
            })
            .unwrap(),
    ))
}

#[derive(Deserialize)]
pub struct SetTeamName {
    name: String,
}

pub async fn route_team_set_name(
    state: State<RouterState>,
    Extension(user): Extension<User>,
    Extension(lang): Extension<Languages>,
    Form(form): Form<SetTeamName>,
) -> Result<impl IntoResponse, StatusCode> {
    if !user.is_team_owner {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut errors = vec![];
    let graphemes = form.name.graphemes(true).count();
    if !(3..=30).contains(&graphemes) {
        errors.push("Team name must be between 3 and 30 characters");
    } else if state
        .db
        .set_team_name(user.team_id, &form.name)
        .await
        .is_err()
    {
        errors.push("Team name already taken");
    }

    let team_name_template = state.jinja.get_template("team-set-name.html").unwrap();

    if errors.is_empty() {
        Ok(Html(
            team_name_template
                .render(context! {
                    lang => lang,
                    new_team_name => &form.name,
                })
                .unwrap(),
        ))
    } else {
        Ok(Html(
            team_name_template
                .render(context! {
                    lang => lang,
                    errors => errors,
                })
                .unwrap(),
        ))
    }
}