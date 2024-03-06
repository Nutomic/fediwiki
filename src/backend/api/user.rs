use crate::{
    backend::{
        database::{read_jwt_secret, IbisData},
        error::MyResult,
    },
    common::{DbPerson, GetUserForm, LocalUserView, LoginUserForm, RegisterUserForm},
};
use activitypub_federation::config::Data;
use anyhow::anyhow;
use axum::{extract::Query, Form, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, Expiration, SameSite};
use axum_macros::debug_handler;
use bcrypt::verify;
use chrono::Utc;
use jsonwebtoken::{
    decode,
    encode,
    get_current_timestamp,
    DecodingKey,
    EncodingKey,
    Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

pub static AUTH_COOKIE: &str = "auth";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// person.username
    pub sub: String,
    /// hostname
    pub iss: String,
    /// Creation time as unix timestamp
    pub iat: i64,
    /// Expiration time
    pub exp: u64,
}

fn generate_login_token(person: &DbPerson, data: &Data<IbisData>) -> MyResult<String> {
    let hostname = data.domain().to_string();
    let claims = Claims {
        sub: person.username.clone(),
        iss: hostname,
        iat: Utc::now().timestamp(),
        exp: get_current_timestamp() + 60 * 60 * 24 * 365,
    };

    let secret = read_jwt_secret(data)?;
    let key = EncodingKey::from_secret(secret.as_bytes());
    let jwt = encode(&Header::default(), &claims, &key)?;
    Ok(jwt)
}

pub async fn validate(jwt: &str, data: &Data<IbisData>) -> MyResult<LocalUserView> {
    let validation = Validation::default();
    let secret = read_jwt_secret(data)?;
    let key = DecodingKey::from_secret(secret.as_bytes());
    let claims = decode::<Claims>(jwt, &key, &validation)?;
    DbPerson::read_local_from_name(&claims.claims.sub, data)
}

#[debug_handler]
pub(in crate::backend::api) async fn register_user(
    data: Data<IbisData>,
    jar: CookieJar,
    Form(form): Form<RegisterUserForm>,
) -> MyResult<(CookieJar, Json<LocalUserView>)> {
    if !data.config.registration_open {
        return Err(anyhow!("Registration is closed").into());
    }
    let user = DbPerson::create_local(form.username, form.password, false, &data)?;
    let token = generate_login_token(&user.person, &data)?;
    let jar = jar.add(create_cookie(token, &data));
    Ok((jar, Json(user)))
}

#[debug_handler]
pub(in crate::backend::api) async fn login_user(
    data: Data<IbisData>,
    jar: CookieJar,
    Form(form): Form<LoginUserForm>,
) -> MyResult<(CookieJar, Json<LocalUserView>)> {
    let user = DbPerson::read_local_from_name(&form.username, &data)?;
    let valid = verify(&form.password, &user.local_user.password_encrypted)?;
    if !valid {
        return Err(anyhow!("Invalid login").into());
    }
    let token = generate_login_token(&user.person, &data)?;
    let jar = jar.add(create_cookie(token, &data));
    Ok((jar, Json(user)))
}

fn create_cookie(jwt: String, data: &Data<IbisData>) -> Cookie<'static> {
    let mut domain = data.domain().to_string();
    // remove port from domain
    if domain.contains(':') {
        domain = domain.split(':').collect::<Vec<_>>()[0].to_string();
    }
    Cookie::build(AUTH_COOKIE, jwt)
        .domain(domain)
        .same_site(SameSite::Strict)
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .expires(Expiration::DateTime(
            OffsetDateTime::now_utc() + Duration::weeks(52),
        ))
        .finish()
}

#[debug_handler]
pub(in crate::backend::api) async fn my_profile(
    data: Data<IbisData>,
    jar: CookieJar,
) -> MyResult<Json<LocalUserView>> {
    let jwt = jar.get(AUTH_COOKIE).map(|c| c.value());
    if let Some(jwt) = jwt {
        Ok(Json(validate(jwt, &data).await?))
    } else {
        Err(anyhow!("invalid/missing auth").into())
    }
}

#[debug_handler]
pub(in crate::backend::api) async fn logout_user(
    data: Data<IbisData>,
    jar: CookieJar,
) -> MyResult<CookieJar> {
    let jar = jar.remove(create_cookie(String::new(), &data));
    Ok(jar)
}

#[debug_handler]
pub(in crate::backend::api) async fn get_user(
    params: Query<GetUserForm>,
    data: Data<IbisData>,
) -> MyResult<Json<DbPerson>> {
    Ok(Json(DbPerson::read_from_name(
        &params.name,
        &params.domain,
        &data,
    )?))
}
