.PHONY: run_app

WEB_NAME := web
MCP_NAME := mcp
ALI_NAME := agent_cli

clean:
	cargo clean
run_agent:
	cargo run --bin $(ALI_NAME)
build_agent:
	cargo build --bin $(ALI_NAME)
run_web:
	cargo run --bin $(WEB_NAME)
build_web:
	cargo build --bin $(WEB_NAME)
run_mcp:
	cargo run --bin $(MCP_NAME)
build_mcp:
	cargo build --bin $(MCP_NAME)
