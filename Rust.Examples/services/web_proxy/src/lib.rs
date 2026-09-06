use std::{
    env,
    io::{self, Write},
};

/// Always asks the user for a port.
///
/// If the environment variable exists, its value becomes the
/// suggested default. Otherwise, fallback_port is used.
pub fn prompt_port(environment_variable: &str, service_name: &str, fallback_port: u16) -> u16 {
    let suggested_port = match env::var(environment_variable) {
        Ok(value) => match value.parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                eprintln!(
                    "Warning: {environment_variable}='{value}' is invalid. \
                     Using {fallback_port} as the default."
                );

                fallback_port
            }
        },
        Err(_) => fallback_port,
    };

    loop {
        print!("Enter port for {service_name} [{suggested_port}]: ");

        io::stdout().flush().expect("failed to flush stdout");

        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("failed to read port");

        let input = input.trim();

        if input.is_empty() {
            return suggested_port;
        }

        match input.parse::<u16>() {
            Ok(port) if port > 0 => return port,
            _ => {
                println!("Invalid port. Enter a number between 1 and 65535.");
            }
        }
    }
}

/// Always asks the user for a text value.
///
/// If the environment variable exists, its value becomes the
/// suggested default.
pub fn prompt_text(environment_variable: &str, message: &str, fallback_value: &str) -> String {
    let suggested_value = env::var(environment_variable)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback_value.to_string());

    print!("{message} [{suggested_value}]: ");

    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("failed to read input");

    let input = input.trim();

    if input.is_empty() {
        suggested_value
    } else {
        input.to_string()
    }
}
