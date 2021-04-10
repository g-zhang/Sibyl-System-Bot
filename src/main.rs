mod commands;
mod userdb;

use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::standard::{
        macros::{group, hook},
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready, id::UserId},
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = Arc<RwLock<HashMap<String, u64>>>;
}

struct MessageCount;

impl TypeMapKey for MessageCount {
    type Value = Arc<AtomicUsize>;
}

pub struct ShardManagerContainer;
type ShardManagerLock = Arc<Mutex<ShardManager>>;
impl TypeMapKey for ShardManagerContainer {
    type Value = ShardManagerLock;
}

pub struct UserDatabase;
type DatabaseLock = Arc<RwLock<userdb::UserProfilesDatabase>>;
impl TypeMapKey for UserDatabase {
    type Value = DatabaseLock;
}

const COMMAND_PREFIX: &str = "$";
const TERMINAL_TITLE_ESCAPE_BEGIN: &str = "\x1b]2;";
const TERMINAL_TITLE_ESCAPE_END: &str = "\x07";

use commands::*;
#[group]
#[commands(analyze, psycho_pass, stats, msg_count, debug)]
struct General;

#[hook]
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Running command '{}' invoked by '{}'",
        command_name,
        msg.author.tag()
    );

    let counter_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<CommandCounter>()
            .expect("Expected CommandCounter in TypeMap.")
            .clone()
    };
    {
        let mut counter = counter_lock.write().await;
        let entry = counter.entry(command_name.to_string()).or_insert(0);
        *entry += 1;
    }

    true
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.content.starts_with(COMMAND_PREFIX) && !msg.author.bot {
            let sentiment_result = userdb::analyze_message(&msg.content);

            let (db_lock, count) = {
                let data_read = ctx.data.read().await;
                (
                    data_read
                        .get::<UserDatabase>()
                        .expect("Expected UserDatabase in TypeMap.")
                        .clone(),
                    data_read
                        .get::<MessageCount>()
                        .expect("Expected MessageCount in TypeMap.")
                        .clone(),
                )
            };
            {
                let mut database = db_lock.write().await;
                database.add_sentiment_result_for_user(
                    &msg.author,
                    sentiment_result,
                );
            }
            count.fetch_add(1, Ordering::SeqCst);
            info!("Recorded message sentiment for {}", msg.author.tag());
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        print!(
            "{}{} - Sibyl System Server{}",
            TERMINAL_TITLE_ESCAPE_BEGIN,
            ready.user.tag(),
            TERMINAL_TITLE_ESCAPE_END
        );
        tracing::subscriber::with_default(
            FmtSubscriber::builder().finish(),
            || {
                info!("\x1b[1m{}\x1b[m is connected!", ready.user.name);
            },
        );
    }
}

fn print_vanity_ascii_header() {
    print!(
        "{}Initializing Sibyl System{}",
        TERMINAL_TITLE_ESCAPE_BEGIN, TERMINAL_TITLE_ESCAPE_END
    );

    print!("\x1b[1m");
    println!(" _____________________    _________________________");
    println!("   S   Y   B   I   L        S   Y   S   T   E   M ");
    println!(" ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾    ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾");
    print!("\x1b[m");
}

fn initialize_enviroment() -> String {
    let _trace =
        tracing::subscriber::set_default(FmtSubscriber::builder().finish());

    win32::enable_ansi_support();
    win32::enable_mitigations();
    print_vanity_ascii_header();

    match dotenv::dotenv() {
        Ok(path) => {
            info!(".env file loaded from '{}'.", path.to_string_lossy())
        }
        Err(error_value) => warn!("Failed to load .env file: {}.", error_value),
    };

    match env::var("RUST_LOG") {
        Ok(_) => {
            let subscriber = FmtSubscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to start the logger");
        }
        Err(_) => {
            tracing::subscriber::set_global_default(
                FmtSubscriber::builder().finish(),
            )
            .expect("Failed to start the logger");
        }
    }

    env::var("DISCORD_TOKEN").expect("Expected a token in the environment")
}

async fn get_owners(
    discord_token: &str,
) -> Result<HashSet<UserId>, serenity::Error> {
    let http = serenity::http::Http::new_with_token(discord_token);
    let info = http.get_current_application_info().await?;
    let mut owners = HashSet::new();
    owners.insert(info.owner.id);

    if let Some(team_info) = info.team {
        for member in team_info.members {
            owners.insert(member.user.id);
        }
    }
    Ok(owners)
}

#[tokio::main]
async fn main() {
    let discord_token = initialize_enviroment();
    let owners = get_owners(&discord_token)
        .await
        .expect("Could not access application info");

    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .with_whitespace(true)
                .prefix(COMMAND_PREFIX)
        })
        .before(before)
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&discord_token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let database = Arc::new(RwLock::new(
        userdb::UserProfilesDatabase::try_create_from_disk(),
    ));
    {
        let mut data = client.data.write().await;

        data.insert::<CommandCounter>(Arc::new(
            RwLock::new(HashMap::default()),
        ));
        data.insert::<MessageCount>(Arc::new(AtomicUsize::new(0)));
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<UserDatabase>(database.clone());
    }

    tokio::spawn(bg_worker(database.clone(), client.shard_manager.clone()));

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

async fn bg_worker(database: DatabaseLock, shard_manager: ShardManagerLock) {
    let duration = std::time::Duration::from_millis(60000);
    loop {
        match tokio::time::timeout(duration, tokio::signal::ctrl_c()).await {
            Ok(Ok(_)) => {
                warn!("Ctrl-C detected, saving db and shutting down.");
                break;
            }
            Ok(Err(ctrlc_err)) => {
                error!("Could not register ctrl+c handler: {:?}", ctrlc_err);
                break;
            }
            Err(_) => {
                debug!("{:?} elapsed, flushing database to disk.", duration);
                database.write().await.to_disk();
            }
        }
    }
    database.write().await.to_disk();
    shard_manager.lock().await.shutdown_all().await;
}
