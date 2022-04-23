use crate::{ext::UserExt, COLOR_GREEN, COLOR_RED};
use chrono::{TimeZone, Utc};
use futures::FutureExt;
use std::{sync::Arc, time::Duration};
use tokio::{task::spawn, time::sleep};
use tracing::{debug, error, warn};
use twilight_gateway::Event;
use twilight_http::Client;
use twilight_mention::{
    timestamp::{Timestamp as TimestampMention, TimestampStyle},
    Mention,
};
use twilight_model::{
    datetime::Timestamp,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
    user::User,
};
use twilight_util::{
    builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource},
    snowflake::Snowflake,
};

#[derive(Clone, serde::Deserialize)]
pub struct Config {
    welcome_channel: u64,
    log_channel: u64,
    patron_role: u64,
    member_role: u64,
    patron_msg: String,
    welcome_msg: String,
}

#[derive(far::Render)]
struct WelcomeData {
    user: String,
}

async fn greet_member(
    http: Arc<Client>,
    guild: Id<GuildMarker>,
    config: Arc<Config>,
    user: Id<UserMarker>,
) -> anyhow::Result<()> {
    let channel = Id::new(config.welcome_channel);
    let member = http.guild_member(guild, user).exec().await?.model().await?;

    let patron_role = Id::new(config.patron_role);
    let member_role = Id::new(config.member_role);

    if member.roles.contains(&member_role) {
        warn!(user = %member.user.as_tuple(), "User already has the member role, skipping");
        return Ok(());
    }

    let data = WelcomeData {
        user: user.mention().to_string(),
    };

    let template = if member.roles.contains(&patron_role) {
        http.add_guild_member_role(guild, user, member_role)
            .exec()
            .await?;

        &config.patron_msg
    } else {
        &config.welcome_msg
    };

    let msg = far::find(template)?.replace(&data);
    http.create_message(channel).content(&msg)?.exec().await?;

    Ok(())
}

fn prepare_embed(user: &User) -> anyhow::Result<EmbedBuilder> {
    let now = chrono::Utc::now();
    let user_id_field = EmbedFieldBuilder::new("ID", user.id.to_string())
        .inline()
        .build();
    let mut embed = EmbedBuilder::new()
        .description(format!("{} ({})", user.as_tuple(), user.id.mention()))
        .field(user_id_field)
        .timestamp(Timestamp::from_secs(now.timestamp())?);

    if let Some(avatar) = user.avatar {
        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=128",
            user.id, avatar
        );
        embed = embed.thumbnail(ImageSource::url(url)?);
    }

    Ok(embed)
}

fn get_config(global: &super::Config, guild: Id<GuildMarker>) -> Option<Arc<Config>> {
    global
        .greet
        .get(&guild.to_string())
        .map(|config| Arc::new(config.clone()))
}

pub async fn handle_join(
    config: &super::Config,
    http: Arc<Client>,
    guild: Id<GuildMarker>,
    user: &User,
) -> anyhow::Result<()> {
    let config = match get_config(config, guild) {
        Some(config) => config,
        None => return Ok(()),
    };
    debug!(user = %user.as_tuple(), %guild, "greet::join");

    {
        let http = http.clone();
        let user = user.id;
        let config = config.clone();

        let fut = sleep(Duration::from_secs(5))
            .then(move |_| greet_member(http, guild, config, user))
            .then(|res| async {
                if let Err(e) = res {
                    error!(error = %e, "greet_member failed");
                }
            });
        spawn(fut);
    }

    let channel = Id::new(config.log_channel);
    let user_creation = Utc.timestamp_millis(user.id.timestamp());
    let user_creation = TimestampMention::new(
        user_creation.timestamp() as u64,
        Some(TimestampStyle::RelativeTime),
    );
    let user_creation_field =
        EmbedFieldBuilder::new("Account created", user_creation.mention().to_string())
            .inline()
            .build();
    let embed = prepare_embed(user)?
        .title("Member joined")
        .color(COLOR_GREEN)
        .field(user_creation_field)
        .build();

    http.create_message(channel)
        .embeds(&[embed])?
        .exec()
        .await?;
    Ok(())
}

pub async fn handle_leave(
    config: &super::Config,
    http: Arc<Client>,
    guild: Id<GuildMarker>,
    user: &User,
) -> anyhow::Result<()> {
    let config = match get_config(config, guild) {
        Some(config) => config,
        None => return Ok(()),
    };
    debug!(user = %user.as_tuple(), %guild, "greet::leave");

    let channel = Id::new(config.log_channel);
    let embed = prepare_embed(user)?
        .title("Member left")
        .color(COLOR_RED)
        .build();

    http.create_message(channel)
        .embeds(&[embed])?
        .exec()
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn debug_handler(
    config: &super::Config,
    http: Arc<Client>,
    event: &Event,
) -> anyhow::Result<()> {
    match event {
        Event::MessageCreate(msg) => {
            if !msg.author.bot {
                if let Some(guild) = msg.guild_id {
                    if msg.content == "ww" {
                        handle_join(config, http, guild, &msg.author).await
                    } else if msg.content == "zz" {
                        handle_leave(config, http, guild, &msg.author).await
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

pub async fn handler(
    config: &super::Config,
    http: Arc<Client>,
    event: &Event,
) -> anyhow::Result<()> {
    match event {
        Event::MemberAdd(event) => {
            if !event.user.bot {
                handle_join(config, http, event.guild_id, &event.user).await
            } else {
                Ok(())
            }
        }
        Event::MemberRemove(event) => {
            if !event.user.bot {
                handle_leave(config, http, event.guild_id, &event.user).await
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}
