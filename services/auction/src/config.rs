use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkName {
    Test,
    Main,
}

impl std::str::FromStr for NetworkName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test" | "testnet" => Ok(NetworkName::Test),
            "main" | "mainnet" => Ok(NetworkName::Main),
            other => Err(format!("unsupported network {other}; use test or main")),
        }
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    /// Print the first N Orchard-only unified addresses derived from a seed file (no chain access).
    Derive {
        #[arg(long)]
        seed_file: PathBuf,
        #[arg(long, default_value_t = 8)]
        count: u32,
        /// Unified addresses to test for ownership by this seed's Orchard incoming viewing key.
        #[arg(long)]
        check: Vec<String>,
    },
    /// Act as a standalone wallet: sync, print address and balance, optionally pay a ZIP-321 URI.
    /// Used for testing bids without a mobile wallet.
    Wallet {
        /// Wallet directory (separate from the service wallet).
        #[arg(long)]
        dir: PathBuf,
        /// Mnemonic file (created if absent).
        #[arg(long)]
        seed_file: PathBuf,
        /// A `zcash:` payment URI to pay after syncing.
        #[arg(long)]
        pay: Option<String>,
        /// Keep syncing until the spendable balance is at least this many zatoshis, then continue.
        #[arg(long)]
        wait_balance: Option<u64>,
    },
}

/// Blind Bazaar auction service.
#[derive(Debug, Parser, Clone)]
#[command(name = "bb-auction")]
pub struct Config {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,

    /// `test` or `main`
    #[arg(long, env = "NETWORK", default_value = "test")]
    pub network: NetworkName,

    /// lightwalletd gRPC endpoint, e.g. https://testnet.zec.rocks:443
    #[arg(long, env = "LIGHTWALLETD_URL", default_value = "https://testnet.zec.rocks:443")]
    pub lightwalletd_url: String,

    /// Directory for wallet DB, block cache and the service DB.
    #[arg(long, env = "DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    /// File holding the operator's 24-word mnemonic (created if absent).
    #[arg(long, env = "OPERATOR_SEED_FILE", default_value = "./data/operator.seed")]
    pub operator_seed_file: PathBuf,

    /// Wallet birthday height used when the wallet is first created.
    #[arg(long, env = "BIRTHDAY_HEIGHT")]
    pub birthday_height: Option<u32>,

    /// Token required in `x-admin-token` for seller/admin routes.
    #[arg(long, env = "ADMIN_TOKEN", default_value = "")]
    pub admin_token: String,

    /// Verify a disclosure JSON file against the chain and exit (no wallet needed).
    #[arg(long)]
    pub verify_disclosure: Option<PathBuf>,

    /// Confirmations required before a received note can be spent (applies to trusted and
    /// untrusted notes alike). Default policy is 3 trusted / 10 untrusted.
    #[arg(long, env = "MIN_CONFIRMATIONS")]
    pub min_confirmations: Option<u32>,

    /// HTTP bind address.
    #[arg(long, env = "BIND", default_value = "127.0.0.1:8787")]
    pub bind: String,

    /// Seconds between sync passes.
    #[arg(long, env = "SYNC_INTERVAL_SECS", default_value_t = 20)]
    pub sync_interval_secs: u64,

    /// Minimum and maximum random delay (seconds) before a refund is sent after close.
    #[arg(long, env = "REFUND_DELAY_MIN_SECS", default_value_t = 30)]
    pub refund_delay_min_secs: u64,
    #[arg(long, env = "REFUND_DELAY_MAX_SECS", default_value_t = 300)]
    pub refund_delay_max_secs: u64,
}

impl Config {
    pub fn wallet_dir(&self) -> PathBuf {
        self.data_dir.join(match self.network {
            NetworkName::Test => "wallet-test",
            NetworkName::Main => "wallet-main",
        })
    }

    pub fn store_path(&self) -> PathBuf {
        self.data_dir.join(match self.network {
            NetworkName::Test => "bb-test.sqlite",
            NetworkName::Main => "bb-main.sqlite",
        })
    }
}
