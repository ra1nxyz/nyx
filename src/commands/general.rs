pub(crate) use crate::types::{Context, Data, Error};

use rand::seq::{IndexedRandom, SliceRandom};
use rand::rng;

use crate::commands::moderation::mod_check;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        say(),
        choose(),
        // add more here
    ]
}

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn say(
    ctx: Context<'_>,
    #[rest]
    text: String, )
-> Result<(), Error> {
    ctx.say(text).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command,)]
pub async fn choose(
    ctx: Context<'_>,
    #[rest]
    options: String,
) -> Result<(), Error> {
    let all_options: Vec<&str> = options.split(',').map(|s| s.trim()).collect();
    println!("{:?}", all_options);

    if all_options.len() < 2 {
            ctx.say("Minimum of 2 options required").await?;
            return Err(format!("Less than required arguments were given for command").into());
    }
    let choice = all_options.choose(&mut rng()).unwrap();
    ctx.say(format!("{}", choice)).await?;
    Ok(())
}


