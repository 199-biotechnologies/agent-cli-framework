use serde::Serialize;

use crate::cli::Style;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct Greeting {
    name: String,
    style: String,
    message: String,
}

pub fn run(ctx: Ctx, name: String, style: Style) -> Result<(), AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::InvalidInput("name cannot be empty".into()));
    }

    let message = match style {
        Style::Friendly => format!("Hey {name}, good to see you!"),
        Style::Formal => format!("Good day, {name}. A pleasure."),
        Style::Pirate => format!("Ahoy, {name}! Welcome aboard!"),
    };

    let greeting = Greeting {
        name,
        style: style.to_string(),
        message,
    };

    output::print_success_or(ctx, &greeting, |g| {
        use owo_colors::OwoColorize;
        println!("{}", g.message.green());
    });

    Ok(())
}
