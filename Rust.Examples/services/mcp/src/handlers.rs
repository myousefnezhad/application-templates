use app_state::AppState;
use app_tools::{
    calculator::CalculatorTools, counter::CounterTools, customer::CustomerTools, rag::RagTools,
    users::UserTools,
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{prompt::PromptContext, tool::ToolCallContext},
    model::*,
    serde_json::Value as JsonValue,
    service::RequestContext,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct McpHandler {
    calculator: CalculatorTools,
    counter: CounterTools,
    customer: CustomerTools,
    rag: RagTools,
    user: UserTools,
}

impl McpHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            calculator: CalculatorTools::new(),
            counter: CounterTools::new(state.clone()),
            customer: CustomerTools::new(state.clone()),
            rag: RagTools::new(state.clone()),
            user: UserTools::new(state.clone()),
        }
    }
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Unified Server".into()),
        }
    }

    async fn initialize(
        &self,
        _req: InitializeRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _req: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = Vec::new();
        // Calculator Tools
        for t in self.calculator.tool_router.list_all() {
            tools.push(Tool {
                output_schema: None,
                ..t
            });
        }
        // Counter Tools
        for t in self.counter.tool_router.list_all() {
            tools.push(Tool {
                output_schema: None,
                ..t
            });
        }
        // Customer Tools
        for t in self.customer.tool_router.list_all() {
            tools.push(Tool {
                output_schema: None,
                ..t
            });
        }
        // Rag Tools
        for t in self.rag.tool_router.list_all() {
            tools.push(Tool {
                output_schema: None,
                ..t
            });
        }
        // User Tools
        for t in self.user.tool_router.list_all() {
            tools.push(Tool {
                output_schema: None,
                ..t
            });
        }
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParams,
        mut ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        ctx.meta.insert(
            "__adk_tool_name".into(),
            JsonValue::String(req.name.clone().to_string()),
        );
        ctx.meta.insert(
            "__adk_tool_args".into(),
            JsonValue::Object(req.arguments.clone().unwrap_or_default()),
        );
        // Check if Calculator owns this tool
        if self.calculator.tool_router.has_route(&req.name) {
            let calc_ctx = ToolCallContext::new(
                &self.calculator,
                req, // Move req here
                ctx,
            );
            return self.calculator.tool_router.call(calc_ctx).await;
        }

        // Check if Counter owns this tool
        if self.counter.tool_router.has_route(&req.name) {
            let counter_ctx = ToolCallContext::new(
                &self.counter,
                req, // Move req here since we are returning immediately
                ctx,
            );
            return self.counter.tool_router.call(counter_ctx).await;
        }

        // Check if Customer owns this tool
        if self.customer.tool_router.has_route(&req.name) {
            let customer_ctx = ToolCallContext::new(
                &self.customer,
                req, // Move req here since we are returning immediately
                ctx,
            );
            return self.customer.tool_router.call(customer_ctx).await;
        }

        // Check if Rag owns this tool
        if self.rag.tool_router.has_route(&req.name) {
            let rag_ctx = ToolCallContext::new(
                &self.rag, req, // Move req here
                ctx,
            );
            return self.rag.tool_router.call(rag_ctx).await;
        }

        // Check if User owns this tool
        if self.user.tool_router.has_route(&req.name) {
            let calc_ctx = ToolCallContext::new(
                &self.user, req, // Move req here
                ctx,
            );
            return self.user.tool_router.call(calc_ctx).await;
        }

        // 3. Tool not found in either router
        Err(McpError {
            code: ErrorCode(-32601), // Method Not Found
            message: format!("Tool '{}' not found", req.name).into(),
            data: None,
        })
    }

    async fn list_prompts(
        &self,
        _req: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        // 1. Get Counter Prompts
        let counter_prompts = self.counter.prompt_router.list_all();

        // 2. Get Calculator Prompts (NEW)
        let calc_prompts = self.calculator.prompt_router.list_all();

        // 3. Merge them
        let mut all_prompts = Vec::new();

        // Helper closure to map internal definitions to public Prompt struct
        let map_prompt = |p: Prompt| Prompt {
            name: p.name,
            description: p.description,
            arguments: p.arguments,
            icons: None,
            title: None,
            meta: None,
        };

        all_prompts.extend(counter_prompts.into_iter().map(map_prompt));
        all_prompts.extend(calc_prompts.into_iter().map(map_prompt));

        Ok(ListPromptsResult {
            prompts: all_prompts,
            next_cursor: None,
            meta: None,
        })
    }

    async fn get_prompt(
        &self,
        req: GetPromptRequestParams,
        ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        // 1. Check if Counter has this prompt
        if self.counter.prompt_router.has_route(&req.name) {
            let prompt_ctx = PromptContext::new(&self.counter, req.name, req.arguments, ctx);
            return self.counter.prompt_router.get_prompt(prompt_ctx).await;
        }

        // 2. Check if Calculator has this prompt
        if self.calculator.prompt_router.has_route(&req.name) {
            let prompt_ctx = PromptContext::new(&self.calculator, req.name, req.arguments, ctx);
            return self.calculator.prompt_router.get_prompt(prompt_ctx).await;
        }

        // 3. If neither has it, return Method Not Found explicitly
        Err(McpError {
            code: ErrorCode(-32601), // Standard JSON-RPC Method Not Found
            message: format!("Prompt '{}' not found", req.name).into(),
            data: None,
        })
    }

    async fn list_resources(
        &self,
        _req: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = self.counter.list_my_resources().await;
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        req: ReadResourceRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let content = self.counter.read_my_resource(&req.uri).await?;
        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, req.uri)],
        })
    }
}
