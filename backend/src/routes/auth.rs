use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::ApiResponse;
use crate::routes::auth_middleware::CurrentUser;
use crate::state::{AppState, StoredUser};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub token: String,
}

fn hash_password(password: &str) -> Result<String, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, StatusCode> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn generate_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, StatusCode> {
    let username = req.username.trim().to_string();
    let password = req.password;
    if username.is_empty() || password.is_empty() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "用户名和密码不能为空".to_string(),
            data: None,
        }));
    }

    let password_hash = hash_password(&password)?;

    let mut users = state.users.lock().await;
    if users.contains_key(&username) {
        return Ok(Json(ApiResponse {
            success: false,
            message: "用户已存在".to_string(),
            data: None,
        }));
    }

    let nickname = req.nickname.unwrap_or_else(|| username.clone());
    let user = StoredUser {
        username: username.clone(),
        nickname: nickname.clone(),
        avatar: req.avatar.clone(),
        password_hash,
    };
    users.insert(username.clone(), user);
    drop(users);
    state.save_users().await;

    let token = generate_token();
    state.sessions.lock().await.insert(token.clone(), username.clone());
    state.save_sessions().await;

    Ok(Json(ApiResponse {
        success: true,
        message: "注册成功".to_string(),
        data: Some(UserResponse {
            username,
            nickname,
            avatar: req.avatar,
            token,
        }),
    }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, StatusCode> {
    let username = req.username.trim().to_string();
    if username.is_empty() || req.password.is_empty() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "用户名和密码不能为空".to_string(),
            data: None,
        }));
    }

    let users = state.users.lock().await;
    let user = match users.get(&username) {
        Some(u) => u.clone(),
        None => {
            return Ok(Json(ApiResponse {
                success: false,
                message: "用户不存在".to_string(),
                data: None,
            }));
        }
    };
    drop(users);

    if !verify_password(&req.password, &user.password_hash)? {
        return Ok(Json(ApiResponse {
            success: false,
            message: "密码错误".to_string(),
            data: None,
        }));
    }

    let token = generate_token();
    state.sessions.lock().await.insert(token.clone(), username.clone());
    state.save_sessions().await;

    Ok(Json(ApiResponse {
        success: true,
        message: "登录成功".to_string(),
        data: Some(UserResponse {
            username: user.username,
            nickname: user.nickname,
            avatar: user.avatar,
            token,
        }),
    }))
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, StatusCode> {
    // Always act on the authenticated user; the profile being updated cannot be
    // chosen by the request body (prevents IDOR).
    let username = current_user.username;

    let mut users = state.users.lock().await;
    let response = match users.get_mut(&username) {
        Some(user) => {
            if let Some(nickname) = req.nickname {
                user.nickname = nickname;
            }
            if let Some(avatar) = req.avatar {
                user.avatar = Some(avatar);
            }
            Some(UserResponse {
                username: user.username.clone(),
                nickname: user.nickname.clone(),
                avatar: user.avatar.clone(),
                token: String::new(),
            })
        }
        None => None,
    };
    drop(users);

    match response {
        Some(data) => {
            state.save_users().await;
            Ok(Json(ApiResponse {
                success: true,
                message: "ok".to_string(),
                data: Some(data),
            }))
        }
        None => Ok(Json(ApiResponse {
            success: false,
            message: "用户不存在".to_string(),
            data: None,
        })),
    }
}

/// Invalidate the caller's session token so it stops accumulating in the
/// in-memory session map. Idempotent: a missing/unknown token still succeeds.
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<ApiResponse<()>> {
    if let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        let removed = state.sessions.lock().await.remove(token).is_some();
        if removed {
            state.save_sessions().await;
        }
    }
    Json(ApiResponse {
        success: true,
        message: "已登出".to_string(),
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState::new())
    }

    #[tokio::test]
    async fn test_register_and_login() {
        let state = test_state();
        let reg = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "alice".to_string(),
                password: "secret".to_string(),
                nickname: Some("Alice".to_string()),
                avatar: None,
            }),
        )
        .await
        .unwrap();
        assert!(reg.0.success);
        assert_eq!(reg.0.data.as_ref().unwrap().nickname, "Alice");

        let login_ok = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "alice".to_string(),
                password: "secret".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(login_ok.0.success);

        let login_bad = login(
            State(state),
            Json(LoginRequest {
                username: "alice".to_string(),
                password: "wrong".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(!login_bad.0.success);
    }

    #[tokio::test]
    async fn test_duplicate_register() {
        let state = test_state();
        let _ = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "bob".to_string(),
                password: "pass".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();

        let dup = register(
            State(state),
            Json(RegisterRequest {
                username: "bob".to_string(),
                password: "other".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();
        assert!(!dup.0.success);
    }

    #[tokio::test]
    async fn test_update_profile() {
        let state = test_state();
        let _ = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "carol".to_string(),
                password: "pass".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();

        let updated = update_profile(
            State(state),
            CurrentUser {
                username: "carol".to_string(),
            },
            Json(UpdateProfileRequest {
                nickname: Some("Carol".to_string()),
                avatar: Some("https://example.com/avatar.png".to_string()),
            }),
        )
        .await
        .unwrap();
        assert!(updated.0.success);
        let user = updated.0.data.unwrap();
        assert_eq!(user.nickname, "Carol");
        assert_eq!(user.avatar, Some("https://example.com/avatar.png".to_string()));
    }
}
