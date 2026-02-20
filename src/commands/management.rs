use poise::{command, serenity_prelude as serenity};
use sqlx::{Column, Execute, Row};
use poise::CreateReply;

use super::moderation::{BOT_OWNER_ID};

pub(crate) use crate::types::{Context, Data, Error};


pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        exec(),
        // add more here
    ]
}

#[poise::command(slash_command, prefix_command)]
pub async fn exec(
    ctx: Context<'_>,
    #[rest]
    query: String,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }
    if query.trim().is_empty() {
        return Err("Empty query string".into());
    }
    let query_type = query.split_whitespace().next().unwrap_or("").to_lowercase();

    match query_type.as_str() {
        "select" | "pragma" | "explain" => {
            match handle_select(&ctx, &query).await {
                Ok(response) => {
                    let file = serenity::CreateAttachment::bytes(response.into_bytes(), "result.txt");
                    ctx.send(CreateReply::default()
                                 .attachment(file)
                    ).await?;
                }
                Err(err) => {
                    ctx.say(format!("Exec failed: {}", err)).await?;
                    return Err(format!("Error handling select command: {}", err).into());
                }
            }
        }
        _ => {
            match handle_update(&ctx, &query).await {
                Ok(rows_affected) => {
                    ctx.send(CreateReply::default()
                        .content(format!("Query result: Rows affected {}", rows_affected))
                    .reply(true)).await?;
                }
                Err(err) => {
                    ctx.say(format!("Exec failed: {}", err)).await?;
                    return Err(format!("Error handling update command: {}", err).into());
                }
            }
        }
    }
    Ok(())
}

async fn handle_select(ctx: &Context<'_>, query: &str) -> Result<String, sqlx::Error> {
    use sqlx::sqlite::SqliteRow;

    let rows: Vec<SqliteRow> = sqlx::query(query)
        .fetch_all(&ctx.data().db)
        .await?;

    if rows.is_empty() {
        return Ok("No results".to_string());
    }

    let mut result = String::new();

    let columns: Vec<String> = rows[0].columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let mut column_width: Vec<usize> = columns.iter().map(|c| c.len()).collect();

    for row in rows.iter().take(10) { // attempt to sample table elements for widths
        for (i, _) in columns.iter().enumerate() {
            let cell_str = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => "NULL".to_string(),
                },
            };
            column_width[i] = column_width[i].max(cell_str.len());
        }
    }

    for (i, col) in columns.iter().enumerate() {
        result.push_str(&format!("{:width$} | ", col, width=column_width[i]));
    }

    result.pop();
    result.pop();
    result.push('\n');

    for &width in &column_width {
        result.push_str(&"-".repeat(width+2));
        result.push('+');
    }
    result.pop();
    result.push('\n');

    for row in rows.iter().take(10) { // limit against chat flood
        for (i, _) in columns.iter().enumerate() {
            let cell_value = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => match row.try_get::<bool, _>(i) {
                        Ok(b) => b.to_string(),
                        Err(_) => "NULL".to_string(),
                    },
                },
            };
            result.push_str(&format!("{:width$} | ", cell_value, width = column_width[i]));
        }
        result.pop();
        result.pop();
        result.push('\n');
    }

    if (rows.len() > 10) {
        result.push_str(&format!("... {} more rows", rows.len()-10))
    }
    Ok((result))
}

async fn handle_update(ctx: &Context<'_>, query: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(query)
        .execute(&ctx.data().db)
        .await?;

    Ok(result.rows_affected())
}
