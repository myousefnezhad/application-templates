use adk_core::{Content, ReadonlyContext};

#[derive(Debug, Default)]
pub struct SimpleContext {
    pub invocation_id: Option<String>,
    pub agent_name: Option<String>,
    pub user_id: Option<String>,
    pub app_name: Option<String>,
    pub session_id: Option<String>,
    pub branch: Option<String>,
}

#[async_trait::async_trait]
impl ReadonlyContext for SimpleContext {
    fn invocation_id(&self) -> &str {
        match &self.invocation_id {
            None => "init",
            Some(x) => x,
        }
    }

    fn agent_name(&self) -> &str {
        match &self.agent_name {
            None => "assistant",
            Some(x) => x,
        }
    }

    fn user_id(&self) -> &str {
        match &self.user_id {
            None => "user",
            Some(x) => x,
        }
    }

    fn app_name(&self) -> &str {
        match &self.app_name {
            None => "app",
            Some(x) => x,
        }
    }

    fn session_id(&self) -> &str {
        match &self.session_id {
            None => "init",
            Some(x) => x,
        }
    }

    fn branch(&self) -> &str {
        match &self.branch {
            None => "main",
            Some(x) => x,
        }
    }

    fn user_content(&self) -> &Content {
        static CONTENT: std::sync::OnceLock<Content> = std::sync::OnceLock::new();
        CONTENT.get_or_init(|| Content::new("user").with_text(""))
    }
}
