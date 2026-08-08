use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PostLoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PostLoginOutput {
    pub access_token: String,
    pub refresh_token: String,
}
