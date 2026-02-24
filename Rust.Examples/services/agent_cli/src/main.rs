use adk_core::Content;
use adk_rust::prelude::Event;
use adk_rust::session::{CreateRequest, SessionService};
use app_agent::{builder::agent_builder, runner::stream_response_parser};
use app_config::AppConfig;
use dotenv::dotenv;
use std::{
    collections::HashMap,
    io::{self, Write},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    // Assume single user application
    let user_id = "console_user".to_string();
    let session_id = Uuid::new_v4().to_string();
    let config = AppConfig::new();
    // Build Agent
    let (runner, session) = agent_builder(&config).await?;

    // Create a session
    let create_req = CreateRequest {
        app_name: config.agent_app_name.clone(),
        user_id: user_id.clone(),
        session_id: Some(session_id.clone()),
        state: HashMap::new(),
    };
    session.create(create_req).await?;

    // Launcher::new(agent).run().await?;

    // Or manually run it
    // Keep Conversation History
    let mut history: Vec<Event> = Vec::new();
    eprintln!(
        "Interactive console.\nexit\t\tQuit Agent\ndebug\t\tShow internal events\nclean\t\tClean internal events"
    );
    // Cli Main Loop
    loop {
        // Read Line
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        // Exit
        if line.eq_ignore_ascii_case("exit") {
            break;
        }
        // Show History
        if line.eq_ignore_ascii_case("debug") {
            for h in &history {
                println!("{:#?}", &h);
            }
            continue;
        }
        // Clean History
        if line.eq_ignore_ascii_case("clean") {
            history = Vec::new();
            continue;
        }

        // Content is created like this (role + parts)
        let input = Content::new("user").with_text(line);

        // Run one turn; stream events
        let mut stream = runner
            .run(user_id.to_string(), session_id.to_string(), input)
            .await?;
        let buf = stream_response_parser(&mut stream, Some(&mut history)).await?;
        println!("{}", &buf);
    }
    Ok(())
}
