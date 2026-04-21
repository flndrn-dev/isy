use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod convex_client;
mod mls_poc;
mod storage;

#[derive(Parser)]
#[command(name = "isy-proto", about = "ISY MLS Week-3 prototype CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register a UIN, generate credential + KeyPackages, publish to Convex.
    Register {
        #[arg(long)]
        uin: u64,
        #[arg(long)]
        db: String,
    },
    /// Add a peer UIN to a group (creates the group on first add).
    Add {
        #[arg(long = "my-uin")]
        my_uin: u64,
        #[arg(long = "peer-uin")]
        peer_uin: u64,
        #[arg(long)]
        db: String,
    },
    /// Send an encrypted message to the group with the given peer.
    Send {
        #[arg(long = "my-uin")]
        my_uin: u64,
        #[arg(long = "peer-uin")]
        peer_uin: u64,
        #[arg(long)]
        message: String,
        #[arg(long)]
        db: String,
    },
    /// Poll inbox, decrypt new messages, print them.
    Inbox {
        #[arg(long = "my-uin")]
        my_uin: u64,
        #[arg(long)]
        db: String,
    },
    /// Remove a peer from a group.
    Remove {
        #[arg(long = "my-uin")]
        my_uin: u64,
        #[arg(long = "peer-uin")]
        peer_uin: u64,
        #[arg(long)]
        db: String,
    },
    /// Hidden: run in-memory MLS proof-of-concept (Alice+Bob, no Convex).
    #[command(hide = true)]
    Poc,
    /// Hidden: smoke-test Convex connectivity by calling uins:lookupUin for a nonexistent UIN.
    #[command(hide = true)]
    Ping,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Register { uin, db } => {
            commands::register::run(uin, &db).await?;
        }
        Command::Add { my_uin, peer_uin, db } => {
            tracing::info!("TODO: add my_uin={} peer_uin={} db={}", my_uin, peer_uin, db);
        }
        Command::Send { my_uin, peer_uin, message, db } => {
            tracing::info!(
                "TODO: send my_uin={} peer_uin={} msg_len={} db={}",
                my_uin, peer_uin, message.len(), db
            );
        }
        Command::Inbox { my_uin, db } => {
            tracing::info!("TODO: inbox my_uin={} db={}", my_uin, db);
        }
        Command::Remove { my_uin, peer_uin, db } => {
            tracing::info!("TODO: remove my_uin={} peer_uin={} db={}", my_uin, peer_uin, db);
        }
        Command::Poc => {
            mls_poc::run_poc()?;
        }
        Command::Ping => {
            let client = convex_client::ConvexClient::from_env()?;
            let result = client
                .query("uins:lookupUin", serde_json::json!({"uin": 999999999}))
                .await?;
            tracing::info!("convex ping result: {:?}", result);
        }
    }

    Ok(())
}
