import uuid
import asyncio
from rich import print_json
from langchain_openai import ChatOpenAI
from langchain_mcp_adapters.client import MultiServerMCPClient
from langchain_mcp_adapters.tools import load_mcp_tools
from langgraph.checkpoint.memory import InMemorySaver
from langchain.agents import create_agent

MCP_URL = "http://localhost:9001/v1/mcp"
MCP_TOKEN = "<MCP TOKEN>"
VLLM_TOKEN = "<VLLM TOKEN>"
VLLM_MODEL = "openai/gpt-oss-20b"
VLLM_BASE_URL = "http://localhost:8000/v1"
SYSTEM_PROMPT = """You are an agent that uses the available tools to answer questions."""

async def main():
    # LangGraph memory thread_id (unrelated to MCP session)
    # thread_id = "20e2308a-98df-11b0-a04e-ff0051748e5a"
    thread_id = str(uuid.uuid4())

    checkpointer = InMemorySaver()

    client = MultiServerMCPClient(
        {
            "mcp": {
                "url": MCP_URL,
                "transport": "streamable_http",
                "headers": {"Authorization": f"Bearer {MCP_TOKEN}"},
            }
        }
    )

    model = ChatOpenAI(
        base_url=VLLM_BASE_URL,
        api_key=VLLM_TOKEN,
        model=VLLM_MODEL,
        temperature=0,
        max_completion_tokens=1000,
    )

    # IMPORTANT: keep ONE MCP session alive for the whole chat loop
    async with client.session("mcp") as session:
        tools = await load_mcp_tools(session)
        print("Loaded MCP tools:", sorted([getattr(t, "name", "<no-name>") for t in tools]))

        agent = create_agent(
            model,
            tools,
            system_prompt=SYSTEM_PROMPT,
            checkpointer=checkpointer,
        )

        config = {"configurable": {"thread_id": thread_id}}
        result = None
        while True:
            user_prompt = input("Please enter your question: ").strip()
            if user_prompt.lower() in {"exit", "quit", "q"}:
                print("Goodbye!")
                return

            if user_prompt.lower() in {"debug", "d"}:
                if result is not None:
                    for i, msg in enumerate(result["messages"]):
                        print(f"\n--- Message {i} ---")
                        print_json(data=msg.dict())
                continue

            result = await agent.ainvoke(
                {"messages": [{"role": "user", "content": user_prompt}]},
                config,
            )
            print(result["messages"][-1].content)

if __name__ == "__main__":
    asyncio.run(main())
