use adk_core::{AdkError, EventStream};
use adk_rust::prelude::{Event, Part};
use futures::StreamExt;

pub async fn stream_response_parser(
    stream: &mut EventStream,
    mut history: Option<&mut Vec<Event>>,
) -> Result<String, AdkError> {
    // Print only the final response content
    let mut buf = String::new();

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let Some(h) = history.as_deref_mut() {
            h.push(ev.clone());
        }
        match &ev.content() {
            Some(ctx) => {
                for part in ctx.parts.iter() {
                    match &part {
                        Part::Text { text } => buf.push_str(&text),
                        _ => (),
                    }
                }
            }
            None => (),
        }
    }
    Ok(buf)
}
