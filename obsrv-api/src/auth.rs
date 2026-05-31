use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};

use crate::errors::ApiError;

/// Authenticated caller — derived from the wallet-signed headers sent by
/// the UI:
///
///     X-User-Id        base58 Solana pubkey  (also acts as user_id)
///     X-User-Auth-Msg  the exact message the wallet signed
///     X-User-Signature base58 ed25519 signature of that message
///
/// For now we treat the `X-User-Id` header as the identifier and trust the
/// caller — the structure is set up so that an `ed25519-dalek` verification
/// can be plugged in here later without changing any handler.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

fn extract_user_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user_id = extract_user_id(&parts.headers).ok_or_else(|| {
            ApiError::Unauthorized(
                "missing X-User-Id header — connect a wallet in the UI to sign in".to_string(),
            )
        })?;

        // TODO: verify X-User-Signature against X-User-Auth-Msg with
        //       ed25519-dalek before considering this user authenticated.

        Ok(AuthUser { user_id })
    }
}

/// Same as `AuthUser` but returns `None` instead of rejecting when the
/// header is missing — handy for endpoints that should work anonymously
/// too (e.g. the public `/health` check).
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<AuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuthUser(
            extract_user_id(&parts.headers).map(|user_id| AuthUser { user_id }),
        ))
    }
}
