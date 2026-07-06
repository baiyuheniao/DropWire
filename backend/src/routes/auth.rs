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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::models::ApiResponse;
use crate::routes::auth_middleware::CurrentUser;
use crate::routes::upload::now_secs;
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

const MAX_LOGIN_FAILURES: u32 = 5;
const LOGIN_LOCKOUT_SECS: u64 = 60;
/// Upper bound on distinct usernames tracked for lockout purposes. Without
/// this, an unauthenticated caller could grow `login_attempts` without bound
/// by POSTing /auth/login with arbitrary nonexistent usernames.
const MAX_TRACKED_LOGIN_USERNAMES: usize = 10_000;

/// Monotonic counter for `LoginAttempt::last_attempt_seq`. A wall-clock
/// timestamp would tie constantly under bursts of requests landing in the
/// same second, making "evict the oldest" pick an arbitrary entry among the
/// ties instead of the one that's genuinely least recently active.
static LOGIN_ATTEMPT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Record a failed login attempt and lock the username out once it has
/// failed too many times in a row, so passwords can't be brute-forced at
/// unlimited speed.
async fn record_failed_login(state: &AppState, username: &str) {
    let mut attempts = state.login_attempts.lock().await;

    if !attempts.contains_key(username) && attempts.len() >= MAX_TRACKED_LOGIN_USERNAMES {
        if let Some(oldest) = attempts
            .iter()
            .min_by_key(|(_, a)| a.last_attempt_seq)
            .map(|(k, _)| k.clone())
        {
            attempts.remove(&oldest);
        }
    }

    let entry = attempts.entry(username.to_string()).or_default();
    entry.failures += 1;
    entry.last_attempt_seq = LOGIN_ATTEMPT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    if entry.failures >= MAX_LOGIN_FAILURES {
        entry.locked_until = Some(now_secs() + LOGIN_LOCKOUT_SECS);
    }
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

    {
        let mut attempts = state.login_attempts.lock().await;
        if let Some(entry) = attempts.get_mut(&username) {
            if let Some(locked_until) = entry.locked_until {
                if locked_until > now_secs() {
                    return Ok(Json(ApiResponse {
                        success: false,
                        message: "登录尝试次数过多，请稍后再试".to_string(),
                        data: None,
                    }));
                }
                // The lockout window has passed: give this username a clean
                // slate rather than letting a stale failure count ratchet
                // the lock forward forever. Without this, an attacker who
                // doesn't even know the password could keep the real owner
                // perpetually locked out by sending one wrong guess per
                // lockout window.
                entry.failures = 0;
                entry.locked_until = None;
            }
        }
    }

    let users = state.users.lock().await;
    let user = match users.get(&username) {
        Some(u) => u.clone(),
        None => {
            drop(users);
            record_failed_login(&state, &username).await;
            return Ok(Json(ApiResponse {
                success: false,
                message: "用户不存在".to_string(),
                data: None,
            }));
        }
    };
    drop(users);

    if !verify_password(&req.password, &user.password_hash)? {
        record_failed_login(&state, &username).await;
        return Ok(Json(ApiResponse {
            success: false,
            message: "密码错误".to_string(),
            data: None,
        }));
    }

    state.login_attempts.lock().await.remove(&username);

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

    #[tokio::test]
    async fn login_locks_out_after_repeated_failures() {
        let state = test_state();
        let _ = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "dave".to_string(),
                password: "correct".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();

        for _ in 0..MAX_LOGIN_FAILURES {
            let attempt = login(
                State(state.clone()),
                Json(LoginRequest {
                    username: "dave".to_string(),
                    password: "wrong".to_string(),
                }),
            )
            .await
            .unwrap();
            assert!(!attempt.0.success);
        }

        // Even the correct password is now rejected until the lockout expires.
        let locked_out = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "dave".to_string(),
                password: "correct".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(!locked_out.0.success);
        assert!(locked_out.0.message.contains("次数过多"));
    }

    #[tokio::test]
    async fn successful_login_clears_failure_count() {
        let state = test_state();
        let _ = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "erin".to_string(),
                password: "correct".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();

        for _ in 0..MAX_LOGIN_FAILURES - 1 {
            let _ = login(
                State(state.clone()),
                Json(LoginRequest {
                    username: "erin".to_string(),
                    password: "wrong".to_string(),
                }),
            )
            .await
            .unwrap();
        }

        let ok = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "erin".to_string(),
                password: "correct".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(ok.0.success);

        assert!(!state
            .login_attempts
            .lock()
            .await
            .contains_key("erin"));
    }

    #[tokio::test]
    async fn lockout_resets_after_expiry_instead_of_ratcheting_forever() {
        let state = test_state();
        let _ = register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "frank".to_string(),
                password: "correct".to_string(),
                nickname: None,
                avatar: None,
            }),
        )
        .await
        .unwrap();

        for _ in 0..MAX_LOGIN_FAILURES {
            let _ = login(
                State(state.clone()),
                Json(LoginRequest {
                    username: "frank".to_string(),
                    password: "wrong".to_string(),
                }),
            )
            .await
            .unwrap();
        }

        // Simulate the lockout window having already passed.
        {
            let mut attempts = state.login_attempts.lock().await;
            attempts.get_mut("frank").unwrap().locked_until = Some(0);
        }

        // A single further wrong guess right after expiry must not
        // immediately re-lock the account - without the fix, `failures` was
        // never reset, so this one guess alone would cross the threshold
        // again and re-trigger a fresh lockout indefinitely.
        let attempt = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "frank".to_string(),
                password: "wrong".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(!attempt.0.success);
        assert!(!attempt.0.message.contains("次数过多"));

        // The real owner can still get in right after.
        let ok = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "frank".to_string(),
                password: "correct".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(ok.0.success);
    }

    #[tokio::test]
    async fn login_attempts_map_is_capped_by_evicting_oldest() {
        let state = test_state();

        // record_failed_login is exercised directly (bypassing register/login,
        // which hash passwords with argon2) so this test can cheaply push the
        // map past its cap.
        for i in 0..=MAX_TRACKED_LOGIN_USERNAMES {
            record_failed_login(&state, &format!("user-{i}")).await;
        }

        let attempts = state.login_attempts.lock().await;
        assert_eq!(attempts.len(), MAX_TRACKED_LOGIN_USERNAMES);
        assert!(
            !attempts.contains_key("user-0"),
            "oldest entry should have been evicted to make room"
        );
        assert!(attempts.contains_key(&format!("user-{MAX_TRACKED_LOGIN_USERNAMES}")));
    }
}
