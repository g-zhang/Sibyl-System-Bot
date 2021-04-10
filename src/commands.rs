use crate::userdb::*;
use crate::{
    CommandCounter, MessageCount, ShardManagerContainer, UserDatabase,
};
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::utils::MessageBuilder;
use serenity::{framework::standard::Args, prelude::*};

#[command]
async fn analyze(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let content = args.rest();
    let sentiment_result = MessageBuilder::new()
        .push_bold_line("Sentiment Analysis")
        .push_codeblock(
            analyze_message(content).to_formatted_json(),
            Some("json"),
        )
        .build();

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.content(sentiment_result);
            m.tts(false);
            m.embed(|e| {
                e.title(msg.author.tag());
                e.description(content);
                e
            });
            m
        })
        .await?;

    Ok(())
}

#[command]
#[aliases("pp", "analyze_user", "scan")]
async fn psycho_pass(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let user_name = match args.single_quoted::<String>() {
        Ok(x) => x,
        Err(_) => {
            msg.reply(ctx, "User name is required for analysis.")
                .await?;
            return Ok(());
        }
    };

    let user_id = match user_name.parse::<UserId>() {
        Ok(id) => id,
        Err(_) => {
            let reply = format!("Failed to parse user name {}", &user_name);
            msg.reply(ctx, reply).await?;
            return Ok(());
        }
    };

    let data = ctx.data.read().await;
    let db_lock = data
        .get::<UserDatabase>()
        .expect("Expected UserDatabase in TypeMap.")
        .clone();

    let cdata = {
        let db = db_lock.read().await;
        if let Some(profile) = db.get_user_profile(&user_id) {
            profile.get_cymatic_data()
        } else {
            let reply = format!("Failed to find user {}", &user_name);
            msg.reply(ctx, reply).await?;
            return Ok(());
        }
    };

    let reply = format!(
        "{} has a crime coefficient of: {:.1}",
        &user_name, cdata.crime_coefficient
    );
    msg.reply(ctx, reply).await?;

    Ok(())
}

#[command]
async fn stats(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let (command_counter, msg_count) = {
        let data_read = ctx.data.read().await;
        let command_counter_lock = data_read
            .get::<CommandCounter>()
            .expect("Expected CommandCounter in TypeMap.")
            .clone();

        let raw_count = data_read
            .get::<MessageCount>()
            .expect("Expected MessageCount in TypeMap.")
            .clone();

        let command_counter = command_counter_lock.read().await;
        let count = raw_count.load(std::sync::atomic::Ordering::Relaxed);

        (command_counter.clone(), count)
    };

    let mut content = MessageBuilder::new();
    content.push_line(format!(
        "System has analyzed {} user message(s)",
        msg_count
    ));
    content.push_line("Command usage counts:");

    for (command, count) in command_counter {
        content.push_line(format!("__{}__: {}", command, count));
    }

    msg.reply(ctx, content).await?;
    Ok(())
}

#[command]
async fn msg_count(ctx: &Context, msg: &Message) -> CommandResult {
    let raw_count = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<MessageCount>()
            .expect("Expected MessageCount in TypeMap.")
            .clone()
    };

    let count = raw_count.load(std::sync::atomic::Ordering::Relaxed);

    msg.reply(
        ctx,
        format!("System has analyzed {} user message(s)", count),
    )
    .await?;

    Ok(())
}

#[command("debug")]
#[sub_commands(debug_user, test_convertcc, quit)]
async fn debug(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "Debug command requires an argument")
        .await?;

    Ok(())
}

#[command]
#[owners_only]
#[aliases("user")]
#[sub_commands(debug_user_dump, debug_user_reset)]
async fn debug_user(
    ctx: &Context,
    msg: &Message,
    _args: Args,
) -> CommandResult {
    msg.reply(&ctx.http, "Debug user command requires an argument")
        .await?;

    Ok(())
}

#[command]
#[owners_only]
#[aliases("dump")]
async fn debug_user_dump(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let user_name = args.single_quoted::<String>()?;
    let user_id = user_name.parse::<UserId>()?;

    let data = ctx.data.read().await;
    let db_lock = data
        .get::<UserDatabase>()
        .expect("Expected UserDatabase in TypeMap.")
        .clone();

    let full_profile = {
        let db = db_lock.read().await;
        if let Some(profile) = db.get_user_profile(&user_id) {
            serde_json::to_string(profile).unwrap()
        } else {
            let reply = format!("Failed to find user {}", &user_name);
            msg.reply(ctx, reply).await?;
            return Ok(());
        }
    };

    msg.reply(
        ctx,
        MessageBuilder::new().push_codeblock(full_profile, Some("json")),
    )
    .await?;
    Ok(())
}

#[command]
#[owners_only]
#[aliases("reset")]
async fn debug_user_reset(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let user_name = args.single_quoted::<String>()?;
    let user_id = user_name.parse::<UserId>()?;

    let data = ctx.data.read().await;
    let db_lock = data
        .get::<UserDatabase>()
        .expect("Expected UserDatabase in TypeMap.")
        .clone();

    {
        let mut db = db_lock.write().await;
        if let Some(profile) = db.get_user_profile_as_mut(&user_id) {
            profile.delete_data();
        } else {
            let reply = format!("Failed to find user {}", &user_name);
            msg.reply(ctx, reply).await?;
            return Ok(());
        }
    }

    msg.reply(ctx, format!("{} profile reset", &user_name))
        .await?;
    Ok(())
}

#[command]
#[aliases("convert")]
async fn test_convertcc(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let value = match args.single_quoted::<f64>() {
        Ok(x) => x,
        Err(_) => {
            msg.reply(ctx, "I require a float to convert.").await?;
            return Ok(());
        }
    };

    let value = ComputedData::convert_compound_to_cc(value);
    msg.reply(ctx, format!("{:.1}", value)).await?;

    Ok(())
}

#[command]
#[owners_only]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let (Some(db), Some(manager)) = (
        data.get::<UserDatabase>(),
        data.get::<ShardManagerContainer>(),
    ) {
        msg.reply(ctx, "Shutting down!").await?;
        db.write().await.to_disk();
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;

        return Ok(());
    }

    Ok(())
}
