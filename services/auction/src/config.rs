use std::path::PathBuf;

use clap::Parser;

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

/// Blind Bazaar auction service.
#[derive(Debug, Parser, Clone)]
#[command(name = "bb-auction")]
pub struct Config {
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
    #[arg(long, env = "ADMIN_TOKEN")]
    pub admin_token: String,

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
