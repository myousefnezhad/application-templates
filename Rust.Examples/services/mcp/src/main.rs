use mcp::mcp_service;

#[tokio::main]
async fn main() {
    mcp_service().await;
}
