use anyhow::{Result, anyhow};
use rmcp::model::CallToolResult;
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    io::{self, Write},
};

pub fn ask(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

pub fn ask_default(prompt: &str, default: &str) -> anyhow::Result<String> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    let s = s.trim();
    if s.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(s.to_string())
    }
}

pub fn print_result(result: &CallToolResult) -> anyhow::Result<()> {
    println!();
    println!("================================");
    println!("RESULT");
    println!("================================");
    println!("is_error: {:?}", result.is_error);
    if let Some(ref structured) = result.structured_content {
        println!();
        println!("Structured Content:");
        println!("{}", serde_json::to_string_pretty(structured)?);
    }
    for (i, c) in result.content.iter().enumerate() {
        println!();
        println!("Content {}", i + 1);
        println!("{:#?}", c);
    }
    Ok(())
}

pub fn ask_from_schema(schema: &Value) -> Result<Value> {
    let mut obj = Map::new();
    let required: HashSet<String> = schema["required"]
        .as_array()
        .map(|x| {
            x.iter()
                .filter_map(|v| v.as_str())
                .map(|x| x.to_string())
                .collect()
        })
        .unwrap_or_default();
    let props = schema["properties"]
        .as_object()
        .ok_or_else(|| anyhow!("No properties"))?;

    for (name, field) in props {
        let t = field["type"].as_str().unwrap_or("string");
        let is_required = required.contains(name);
        println!();
        println!(
            "{} ({}) {}",
            name,
            t,
            if is_required {
                "[required]"
            } else {
                "[optional]"
            }
        );
        let txt = ask("> ")?;
        if txt.is_empty() {
            if is_required {
                anyhow::bail!("{} is required", name);
            } else {
                continue;
            }
        }
        let v = match t {
            "string" => Value::String(txt),
            "integer" => Value::Number(txt.parse::<i64>()?.into()),
            "number" => serde_json::to_value(txt.parse::<f64>()?)?,
            "boolean" => Value::Bool(txt.parse::<bool>()?),
            _ => Value::String(txt),
        };
        obj.insert(name.to_string(), v);
    }
    Ok(Value::Object(obj))
}
