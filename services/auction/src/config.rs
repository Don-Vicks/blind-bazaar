use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkName {
    Test,
    Main,
    Regtest,
}

impl std::str::FromStr for NetworkName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test" | "testnet" => Ok(NetworkName::Test),
            "main" | "mainnet" => Ok(NetworkName::Main),
            "regtest" | "local" => Ok(NetworkName::Regtest),
            other => Err(format!("unsupported network {other}; use test or main")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    All,
    Viewer,
    Spender,
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Role::All),
            "viewer" => Ok(Role::Viewer),
            "spender" => Ok(Role::Spender),
            other => Err(format!("unknown role {other}; use all, viewer or spender")),
        }
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum DkgStep {
    Round1 {
        /// This participant's number, 1..=max.
        #[arg(long)]
        id: u16,
        #[arg(long, default_value_t = 3)]
        max: u16,
        #[arg(long, default_value_t = 2)]
        min: u16,
        /// Working directory; secrets are written here with mode 0600.
        #[arg(long)]
        dir: PathBuf,
    },
    Round2 {
        #[arg(long)]
        id: u16,
        #[arg(long)]
        dir: PathBuf,
    },
    Finish {
        #[arg(long)]
        id: u16,
        #[arg(long)]
        dir: PathBuf,
    },
    /// Build `escrow.json` (group key, minted nk/rivk, public package) from this host's finished share.
    Escrow {
        #[arg(long)]
        id: u16,
        #[arg(long)]
        dir: PathBuf,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum FrostCmd {
    /// Trusted-dealer key generation: writes `<out>/signer-{1,2,3}.json` (each holds one share) and
    /// `<out>/escrow.json` (the group `ak`, minted `nk`/`rivk`, and the encoded full viewing key).
    Keygen {
        #[arg(long, default_value_t = 3)]
        max: u16,
        #[arg(long, default_value_t = 2)]
        min: u16,
        #[arg(long)]
        out: PathBuf,
    },
    /// Distributed key generation, one participant per host, exchanged as files (no dealer ever holds the key).
    /// Order: every host runs `round1`; share the `dkg-N.round1.json` files; every host runs `round2`; send
    /// each `dkg-N.to-M.round2.json` to host M privately; every host runs `finish`; one host runs `escrow`.
    Dkg {
        #[command(subcommand)]
        step: DkgStep,
    },
    /// Run a signer host: holds one share, inspects and co-signs every PCZT the spender submits.
    Signer {
        #[arg(long)]
        share: PathBuf,
        #[arg(long)]
        escrow: PathBuf,
        #[arg(long, env = "FROST_SIGNER_BIND", default_value = "127.0.0.1:8801")]
        bind: String,
        /// Shared secret the spender must present (header `x-frost-token`).
        #[arg(long, env = "FROST_TOKEN")]
        token: Option<String>,
        /// Refuse transactions paying more than this (zatoshis) to addresses the escrow does not own.
        #[arg(long, env = "FROST_MAX_EXTERNAL_ZATS")]
        max_external_zats: Option<u64>,
    },
    /// Print the unified full viewing key and first address for an escrow file.
    Show {
        #[arg(long)]
        escrow: PathBuf,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    /// FROST threshold custody tooling.
    Frost {
        #[command(subcommand)]
        cmd: FrostCmd,
    },
    /// Recompute an auditable lot's result from the chain with its published viewing key, and compare it with
    /// the published result. Needs no wallet of your own; pick any lightwalletd you trust.
    Audit {
        #[arg(long)]
        ufvk: String,
        #[arg(long)]
        birthday: u32,
        /// Lot definition: a file or the `GET /auctions/{id}` URL.
        #[arg(long)]
        lot: String,
        /// Published result: a file or the `GET /auctions/{id}/result` URL. Optional; without it the tool only prints.
        #[arg(long)]
        result: Option<String>,
        /// Scratch directory for the throwaway view-only wallet.
        #[arg(long, default_value = "./audit-wallet")]
        dir: PathBuf,
    },
    /// Simulate how linkable refunds are to lots by timing, naive versus batched (prints a Markdown table).
    SimulateRefunds {
        #[arg(long, default_value_t = 4)]
        losers_per_lot: u32,
        #[arg(long, default_value_t = 600)]
        delay_min: i64,
        #[arg(long, default_value_t = 14_400)]
        delay_max: i64,
        #[arg(long, default_value_t = 600)]
        window_secs: i64,
        #[arg(long, default_value_t = 8)]
        batch_max: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Print the first N Orchard-only unified addresses derived from a seed file (no chain access).
    Derive {
        #[arg(long)]
        seed_file: PathBuf,
        #[arg(long, default_value_t = 8)]
        count: u32,
        /// Unified addresses to test for ownership by this seed's Orchard incoming viewing key.
        #[arg(long)]
        check: Vec<String>,
        /// Print the unified full viewing key (for configuring a viewer process).
        #[arg(long)]
        ufvk: bool,
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

    /// `test`, `main` or `regtest`
    #[arg(long, env = "NETWORK", default_value = "test")]
    pub network: NetworkName,

    /// Regtest only: block height at which NU5 and every later upgrade activate (pre-NU5 upgrades
    /// activate at height 1). Must match the local zebrad configuration.
    #[arg(long, env = "REGTEST_NU_HEIGHT", default_value_t = 1)]
    pub regtest_nu_height: u32,

    /// Regtest only: latest active network upgrade (nu5, nu6, nu6_1, nu6_2, nu6_3, nu7).
    #[arg(long, env = "REGTEST_LATEST_NU", default_value = "nu6_1")]
    pub regtest_latest_nu: String,

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

    /// Process role: `all` (one process holds both keys), `viewer` (viewing key only: ingests bids,
    /// runs auctions, serves the API) or `spender` (spending key: mirrors auctions from the viewer,
    /// recomputes outcomes from its own wallet and signs payouts).
    #[arg(long, env = "ROLE", default_value = "all")]
    pub role: Role,

    /// Viewer role: the operator's unified full viewing key (`uview1…` / `uviewtest1…`).
    #[arg(long, env = "OPERATOR_UFVK")]
    pub operator_ufvk: Option<String>,

    /// Spender role: base URL of the viewer, e.g. http://127.0.0.1:8787.
    #[arg(long, env = "VIEWER_URL")]
    pub viewer_url: Option<String>,

    /// Seconds a winner has to pay the balance after close before forfeiting the deposit.
    /// FROST escrow file (from `frost keygen`). When set, the spender imports the escrow's viewing
    /// key instead of deriving an account from OPERATOR_SEED_FILE and every spend is co-signed.
    #[arg(long, env = "FROST_ESCROW")]
    pub frost_escrow: Option<PathBuf>,

    /// Signer host URLs, comma separated (at least `threshold` must be reachable to spend). Each entry may
    /// carry its own credential as `url|token`, so a compromised signer cannot impersonate the spender
    /// toward another; a bare url uses FROST_TOKEN.
    #[arg(long, env = "FROST_SIGNERS", value_delimiter = ',')]
    pub frost_signers: Vec<String>,

    /// Signer share files to run in-process (development only; no separation from the spender).
    #[arg(long, env = "FROST_SIGNER_SHARES", value_delimiter = ',')]
    pub frost_signer_shares: Vec<PathBuf>,

    /// Shared secret presented to signer hosts.
    #[arg(long, env = "FROST_TOKEN")]
    pub frost_token: Option<String>,

    #[arg(long, env = "SETTLEMENT_WINDOW_SECS", default_value_t = 86_400)]
    pub settlement_window_secs: i64,

    /// Optional URL that receives a JSON POST for every auction event.
    #[arg(long, env = "WEBHOOK_URL")]
    pub webhook_url: Option<String>,

    /// Allowed browser origin for the API (CORS). Default allows any origin, for development.
    #[arg(long, env = "CORS_ORIGIN")]
    pub cors_origin: Option<String>,

    /// Behind a reverse proxy that sets `x-forwarded-for`: rate-limit by that address instead of the socket peer.
    #[arg(long, env = "TRUST_PROXY", default_value_t = false)]
    pub trust_proxy: bool,

    /// Zatoshis the operator keeps in reserve for fees; `/health` warns below pending payouts + this.
    #[arg(long, env = "FEE_RESERVE_ZATS", default_value_t = 1_000_000)]
    pub fee_reserve_zats: u64,

    /// Also disclose the symmetric key of each winning note so anyone can decrypt its memo and check
    /// the bid met the reserve. Reveals the winner's bid and refund address; off by default.
    #[arg(long, env = "DISCLOSE_WINNING_MEMO", default_value_t = false)]
    pub disclose_winning_memo: bool,

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

    /// Refund send times are rounded up to a multiple of this, so refunds from every lot that fall in the
    /// same window leave in one transaction. Larger windows mix more lots; 0 disables batching.
    #[arg(long, env = "REFUND_WINDOW_SECS", default_value_t = 60)]
    pub refund_window_secs: u64,

    /// Most refunds placed in one transaction.
    #[arg(long, env = "REFUND_BATCH_MAX", default_value_t = 8)]
    pub refund_batch_max: usize,
}

impl Config {
    pub fn wallet_dir(&self) -> PathBuf {
        self.data_dir.join(match self.network {
            NetworkName::Test => "wallet-test",
            NetworkName::Main => "wallet-main",
            NetworkName::Regtest => "wallet-regtest",
        })
    }

    pub fn store_path(&self) -> PathBuf {
        self.data_dir.join(match self.network {
            NetworkName::Test => "bb-test.sqlite",
            NetworkName::Main => "bb-main.sqlite",
            NetworkName::Regtest => "bb-regtest.sqlite",
        })
    }
}

impl Config {
    pub fn net(&self) -> crate::wallet::net::Net {
        use crate::wallet::net::Net;
        match self.network {
            NetworkName::Test => Net::Test,
            NetworkName::Main => Net::Main,
            NetworkName::Regtest => Net::regtest(self.regtest_nu_height, &self.regtest_latest_nu),
        }
    }

    pub fn network_name(&self) -> &'static str {
        match self.network {
            NetworkName::Test => "test",
            NetworkName::Main => "main",
            NetworkName::Regtest => "regtest",
        }
    }
}
