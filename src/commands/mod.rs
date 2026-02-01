pub mod general;
pub mod moderation;

use crate::types::{Context, Data, Error};
use poise::Command;

pub fn all_commands() -> Vec<Command<Data, Error>> {
    let mut commands = Vec::new();

    commands.extend(moderation::all_commands());

    //commands.extend(general::all_commands());

    commands
}
