pub mod general;
pub mod moderation;
mod management;

mod fun;

use crate::types::{Context, Data, Error};
use poise::Command;

pub fn all_commands() -> Vec<Command<Data, Error>> {
    let mut commands = Vec::new();

    commands.extend(moderation::all_commands());
    commands.extend(general::all_commands());
    commands.extend(management::all_commands());
    commands.extend(fun::all_commands());



    commands
}

fn clean_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c,
            '\u{2066}'..='\u{2069}' |
            '\u{200B}' | '\u{200C}' | '\u{200D}' |
            '\u{FEFF}' // Byte order mark
        ))
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

// please work useless fuck you test man who had a broken discord client ^ ^ ^ ^
