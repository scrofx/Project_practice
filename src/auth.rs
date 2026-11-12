use argon2::{
    password_hash::{phc::PasswordHash, phc::SaltString, PasswordHasher, PasswordVerifier},
    Argon2,
};
use chrono::Utc;
use diesel::{prelude::*, result::DatabaseErrorKind};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
    response::status,
    serde::json::Json,
};
use campuslost_shared::{UserDto, RegisterRequest, LoginRequest, ContactMethod, ApiError, AuthResponse, Claims};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api_error, db::Db, models::{DBContactMethod, NewUser, User, DBUserRole}, schema::users};

const DEFAULT_TOKEN_TTL_SECONDS: u64 = 60 * 60;

impl From<&User> for UserDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            role: user.role.into(),
            phone: user.phone.clone(),
            telegram_handle: user.telegram_handle.clone(),
            vk_handle: user.vk_handle.clone(),
            preferred_contact: user.preferred_contact.into(),
            avatar_path: user.avatar_path.clone(),
            bio: user.bio.clone(),
            is_active: user.is_active,
            notifications_enabled: user.notifications_enabled,
            rating_sum: user.rating_sum,
            rating_count: user.rating_count,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
}

#[derive(Debug)]
pub struct StudentGuard(pub AuthUser);

#[derive(Debug)]
pub struct ModeratorGuard(pub AuthUser);

#[derive(Debug)]
pub struct AdminGuard(pub AuthUser);

fn role_as_str(role: DBUserRole) -> &'static str {
    match role {
        DBUserRole::Student => "student",
        DBUserRole::Moderator => "moderator",
        DBUserRole::Admin => "admin",
    }
}


fn jwt_secret() -> Result<Vec<u8>, status::Custom<Json<ApiError>>> {
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| api_error(Status::InternalServerError, "configuration_error", "JWT_SECRET is not configured"))?;
    if secret.len() < 32 {
        return Err(api_error(
            Status::InternalServerError,
            "configuration_error",
            "JWT_SECRET must contain at least 32 bytes",
        ));
    }
    Ok(secret.into_bytes())
}

fn token_ttl() -> u64 {
    std::env::var("JWT_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_TOKEN_TTL_SECONDS)
}

fn issue_token(user: &User) -> Result<String, status::Custom<Json<ApiError>>> {
    let secret = jwt_secret()?;
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.to_string(),
        role: role_as_str(user.role).to_owned(),
        iat: now,
        exp: now + token_ttl() as usize,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
        .map_err(|_| api_error(Status::InternalServerError, "token_error", "Could not issue access token"))
}

fn auth_response(user: &User) -> Result<AuthResponse, status::Custom<Json<ApiError>>> {
    Ok(AuthResponse {
        access_token: issue_token(user)?,
        token_type: "Bearer".parse().unwrap(),
        user: UserDto::from(user),
    })
}

fn hash_password(password: &str) -> Result<String, status::Custom<Json<ApiError>>> {
    //hash_password salts automatically 
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| api_error(Status::InternalServerError, "password_error", "Could not secure password"))
}

fn verify_password(password: &str, encoded_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(encoded_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[rocket::post("/auth/register", format = "json", data = "<payload>")]
pub async fn register(
    db: Db,
    payload: Json<RegisterRequest>,
) -> Result<status::Custom<Json<AuthResponse>>, status::Custom<Json<ApiError>>> {
    let email = payload.email.trim().to_owned();
    let full_name = payload.full_name.trim().to_owned();

    if email.is_empty() || !email.contains('@') {
        return Err(api_error(Status::BadRequest, "invalid_email", "A valid email address is required"));
    }
    if full_name.is_empty() {
        return Err(api_error(Status::BadRequest, "invalid_full_name", "Full name is required"));
    }
    if payload.password.len() < 8 {
        return Err(api_error(Status::BadRequest, "weak_password", "Password must contain at least 8 characters"));
    }

    let preferred_contact = payload.preferred_contact.unwrap_or(ContactMethod::Email);
    let phone = payload.phone.as_ref().map(|value| value.trim().to_owned()).filter(|value| !value.is_empty());
    let telegram_handle = payload.telegram_handle.as_ref().map(|value| value.trim().trim_start_matches('@').to_owned()).filter(|value| !value.is_empty());
    let vk_handle = payload.vk_handle.as_ref().map(|value| value.trim().to_owned()).filter(|value| !value.is_empty());

    let contact_is_available = match preferred_contact {
        ContactMethod::Email => true,
        ContactMethod::Phone => phone.is_some(),
        ContactMethod::Telegram => telegram_handle.is_some(),
        ContactMethod::Vk => vk_handle.is_some(),
    };
    if !contact_is_available {
        return Err(api_error(
            Status::BadRequest,
            "invalid_preferred_contact",
            "The preferred contact method must have a corresponding contact value",
        ));
    }

    let password_hash = hash_password(&payload.password)?;
    let new_user = NewUser {
        email,
        password_hash,
        full_name,
        // Public registration can never grant elevated privileges.
        role: Some(DBUserRole::Student),
        phone,
        telegram_handle,
        vk_handle,
        preferred_contact: Some(preferred_contact.into()),
        avatar_path: None,
        bio: None,
        is_active: Some(true),
        notifications_enabled: Some(true),
    };

    let inserted = db
        .run(move |conn| {
            diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<User>(conn)
        })
        .await;

    let user = match inserted {
        Ok(user) => user,
        Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            return Err(api_error(Status::Conflict, "email_already_registered", "An account with this email already exists"));
        }
        Err(_) => {
            return Err(api_error(Status::InternalServerError, "database_error", "Could not create account"));
        }
    };

    let response = auth_response(&user)?;
    Ok(status::Custom(Status::Created, Json(response)))
}

#[rocket::post("/auth/login", format = "json", data = "<payload>")]
pub async fn login(
    db: Db,
    payload: Json<LoginRequest>,
) -> Result<Json<AuthResponse>, status::Custom<Json<ApiError>>> {
    let email = payload.email.trim().to_owned();
    let password = payload.password.clone();

    let found = db
        .run(move |conn| users::table.filter(users::email.eq(email)).first::<User>(conn))
        .await;

    let user = match found {
        Ok(user) => user,
        Err(_) => {
            return Err(api_error(Status::Unauthorized, "invalid_credentials", "Email or password is incorrect"));
        }
    };

    if !user.is_active || !verify_password(&password, &user.password_hash) {
        return Err(api_error(Status::Unauthorized, "invalid_credentials", "Email or password is incorrect"));
    }

    Ok(Json(auth_response(&user)?))
}

#[rocket::get("/users/me")]
pub async fn me(auth: AuthUser) -> Json<UserDto> {
    Json(UserDto::from(&auth.user))
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(header) = request.headers().get_one("Authorization") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        if token.is_empty() {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        let secret = match std::env::var("JWT_SECRET") {
            Ok(secret) if secret.len() >= 32 => secret,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };
        let decoded = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(decoded) => decoded,
            Err(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        let user_id = match Uuid::parse_str(&decoded.claims.sub) {
            Ok(id) => id,
            Err(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        let db = match request.guard::<Db>().await {
            Outcome::Success(db) => db,
            Outcome::Error((status, _)) => return Outcome::Error((status, ())),
            Outcome::Forward(status) => return Outcome::Forward(status),
        };

        let result = db
            .run(move |conn| {
                users::table
                    .find(user_id)
                    .select(User::as_select())
                    .first::<User>(conn)
            })
            .await;

        match result {
            Ok(user) if user.is_active && role_as_str(user.role) == decoded.claims.role => {
                Outcome::Success(AuthUser { user })
            }
            Ok(_) | Err(diesel::result::Error::NotFound) => {
                Outcome::Error((Status::Unauthorized, ()))
            }
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for StudentGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<AuthUser>().await {
            Outcome::Success(auth) if auth.user.role == DBUserRole::Student => {
                Outcome::Success(StudentGuard(auth))
            }
            Outcome::Success(_) => Outcome::Error((Status::Forbidden, ())),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ModeratorGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<AuthUser>().await {
            Outcome::Success(auth)
            if matches!(auth.user.role, DBUserRole::Moderator | DBUserRole::Admin) =>
                {
                    Outcome::Success(ModeratorGuard(auth))
                }
            Outcome::Success(_) => Outcome::Error((Status::Forbidden, ())),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<AuthUser>().await {
            Outcome::Success(auth) if auth.user.role == DBUserRole::Admin => {
                Outcome::Success(AdminGuard(auth))
            }
            Outcome::Success(_) => Outcome::Error((Status::Forbidden, ())),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}
