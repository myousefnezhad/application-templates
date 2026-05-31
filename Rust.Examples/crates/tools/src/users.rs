#![allow(dead_code)]
use app_dto::auth::user::{McpGetUserByEmailInput, McpGetUserByEmailOutput, McpGetUsersOutput};
use app_error::AppError;
use app_schema::auth::users::User;
use app_state::AppState;
use rmcp::{
    ErrorData as McpError,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    tool, tool_router,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct UserTools {
    state: Arc<AppState>,
    pub tool_router: ToolRouter<Self>,
}

#[tool_router]
impl UserTools {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search user by provided email address")]
    async fn get_user_by_email(
        &self,
        Parameters(args): Parameters<McpGetUserByEmailInput>,
    ) -> Result<Json<McpGetUserByEmailOutput>, McpError> {
        let pg = self.state.pg.clone();
        let result =
            sqlx::query_as::<_, User>(&format!("{} WHERE email = $1", &User::select_query()))
                .bind(&args.email)
                .fetch_one(&pg)
                .await
                .map_err(AppError::from)?;
        Ok(Json(McpGetUserByEmailOutput { result }))
    }

    #[tool(description = "Shows list of all users")]
    async fn get_users(&self) -> Result<Json<McpGetUsersOutput>, McpError> {
        let pg = self.state.pg.clone();
        let results = sqlx::query_as::<_, User>(User::select_query())
            .fetch_all(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(Json(McpGetUsersOutput { results }))
    }
}
