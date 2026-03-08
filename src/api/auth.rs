use crate::app::AppState;
use crate::app::auth::{Principal, get_jwt};
use crate::app::error::{ApiError, ApiResult};
use crate::app::middleware::get_auth_layer;
use crate::app::response::ApiResponse;
use crate::app::utils::verify_password;
use crate::app::valid::ValidJson;
use crate::entity::prelude::*;
use crate::entity::sys_user;
use axum::extract::{ConnectInfo, State};
use axum::{Extension, Router, debug_handler, routing};
use sea_orm::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use validator::Validate;

