use app_schema::auth::users::User;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PostUserInput {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PatchChangePasswordInput {
    pub email: String,
    pub password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpGetUserByEmailInput {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpGetUserByEmailOutput {
    pub result: User,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpGetUsersOutput {
    pub results: Vec<User>,
}
