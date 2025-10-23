use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use askama::Template;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

use crate::auth;

/// Template for 401 Unauthorized error page
#[derive(Template)]
#[template(path = "401.html")]
struct Error401Template {}

/// Middleware for JWT authentication
pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            // Extract token from Authorization header
            let token = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| {
                    if h.starts_with("Bearer ") {
                        Some(&h[7..])
                    } else {
                        None
                    }
                });

            // Extract token from cookie as fallback
            let token_from_cookie = req.cookie("auth_token").map(|c| c.value().to_string());
            let token = token.or(token_from_cookie.as_deref());

            match token {
                Some(token) => {
                    // Validate token
                    match auth::validate_token(token) {
                        Ok(claims) => {
                            // Store claims in request extensions for handlers to access
                            req.extensions_mut().insert(claims);
                            service.call(req).await
                        }
                        Err(_) => {
                            // Invalid token - return 401 with HTML template
                            Err(render_401_error())
                        }
                    }
                }
                None => {
                    // No token provided - return 401 with HTML template
                    Err(render_401_error())
                }
            }
        })
    }
}

/// Extractor for authenticated user information
/// Use this in handler parameters to ensure the request is authenticated
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: uuid::Uuid,
    pub username: String,
}

impl actix_web::FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        match req.extensions().get::<auth::Claims>() {
            Some(claims) => {
                match uuid::Uuid::parse_str(&claims.sub) {
                    Ok(user_id) => {
                        ready(Ok(AuthenticatedUser {
                            user_id,
                            username: claims.username.clone(),
                        }))
                    }
                    Err(_) => {
                        ready(Err(actix_web::error::ErrorUnauthorized("Invalid user ID in token")))
                    }
                }
            }
            None => {
                ready(Err(actix_web::error::ErrorUnauthorized("Authentication required")))
            }
        }
    }
}

/// Optional authenticated user - doesn't fail if not authenticated
/// Use this when you want to check if a user is logged in but don't require it
#[derive(Debug, Clone)]
pub struct OptionalAuth {
    pub user: Option<AuthenticatedUser>,
}

impl actix_web::FromRequest for OptionalAuth {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // First, try to get claims from extensions (set by Authentication middleware)
        let user = req.extensions().get::<auth::Claims>().and_then(|claims| {
            uuid::Uuid::parse_str(&claims.sub).ok().map(|user_id| {
                AuthenticatedUser {
                    user_id,
                    username: claims.username.clone(),
                }
            })
        });

        // If not found, try to extract and validate token from cookie
        let user = user.or_else(|| {
            req.cookie("auth_token")
                .and_then(|cookie| {
                    auth::validate_token(cookie.value()).ok()
                })
                .and_then(|claims| {
                    uuid::Uuid::parse_str(&claims.sub).ok().map(|user_id| {
                        AuthenticatedUser {
                            user_id,
                            username: claims.username.clone(),
                        }
                    })
                })
        });

        ready(Ok(OptionalAuth { user }))
    }
}

/// Helper function to render 401 error page
fn render_401_error() -> Error {
    let template = Error401Template {};
    match template.render() {
        Ok(html) => {
            let response = HttpResponse::Unauthorized()
                .content_type("text/html; charset=utf-8")
                .body(html);
            actix_web::error::InternalError::from_response("", response).into()
        }
        Err(_) => {
            // Fallback to plain text if template rendering fails
            actix_web::error::ErrorUnauthorized("Authentication required")
        }
    }
}
