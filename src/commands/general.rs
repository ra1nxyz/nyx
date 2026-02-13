use poise::CreateReply;
pub(crate) use crate::types::{Context, Data, Error};

use rand::seq::{IndexedRandom, SliceRandom};
use rand::rng;

use crate::commands::moderation::mod_check;
use crate::structs::time_parse::ParsedDuration;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        say(),
        choose(),
        remind(),
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

#[poise::command(slash_command, prefix_command)]
pub async fn remind(
    ctx: Context<'_>,
    message: String,
    #[rest]
    when: String,
) -> Result<(), Error> {
    let parsed = ParsedDuration::new(&when)
        .map_err(|e| format!("Could not parse when: {:?}", e))?;

    let remind_at = parsed.until_datetime();
    println!("test");

    let context  = match ctx {
        poise::Context::Prefix(ctx) => {
            ctx.msg.link()
        }
        _ => {""}.parse()?
    };

    let remind = crate::structs::reminders::Reminder::new(
        ctx.author().id.to_string(),
        remind_at,
        message.to_string(),
        Option::from(context),
    );

    let reminder_id = ctx.data().reminders.add_reminder(&remind).await?;

    ctx.send(CreateReply::default()
        .content(format!("Reminder ID #{} set for {}", reminder_id, parsed.human_readable())
        ).reply(true)
    ).await?;

    Ok(())
}
/*
do later i cba
#[poise::command(slash_command, prefix_command)]
pub async fn reminders(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let reminders = ctx.data().reminders.
}

*/