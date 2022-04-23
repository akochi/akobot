mod greet;

use crate::ext::UserExt;
use futures::{future::BoxFuture, Future, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};
use twilight_gateway::{Event, Intents, Shard};
use twilight_http::Client;

pub async fn log_event(_: &Config, _: Arc<Client>, event: &Event) -> anyhow::Result<()> {
    match event {
        Event::Ready(ready) => {
            let user = &ready.user;
            info!(user = %user.as_tuple(), "Ready");

            Ok(())
        }
        _ => Ok(()),
    }
}

trait EventHandler<'a> {
    fn call(
        &'a self,
        config: &'a Config,
        http: Arc<Client>,
        event: &'a Event,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
}

impl<'a, F, Fut> EventHandler<'a> for F
where
    F: Fn(&'a Config, Arc<Client>, &'a Event) -> Fut,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'a,
{
    fn call(
        &'a self,
        config: &'a Config,
        http: Arc<Client>,
        event: &'a Event,
    ) -> BoxFuture<anyhow::Result<()>> {
        Box::pin(self(config, http, event))
    }
}

const HANDLERS: &[&dyn for<'r> EventHandler<'r>] = &[&log_event, &greet::handler];

pub async fn dispatch_event(
    config: &Config,
    http: Arc<Client>,
    event: &Event,
) -> anyhow::Result<()> {
    for handler in HANDLERS {
        handler.call(config, http.clone(), event).await?;
    }

    Ok(())
}

#[derive(serde::Deserialize)]
pub struct Config {
    greet: HashMap<String, greet::Config>,
}

pub async fn run(config: Config, token: &str) {
    let (shard, mut events) = Shard::builder(
        token.to_string(),
        Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    )
    .build();
    shard.start().await.unwrap();

    let http = Arc::new(Client::builder().token(token.to_string()).build());

    while let Some(event) = events.next().await {
        if let Err(e) = dispatch_event(&config, http.clone(), &event).await {
            error!(error = %e, event = ?event.kind(), "Event handler failed");
        }
    }
}
