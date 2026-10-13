//! Network parameters: main, test, or a regtest with caller-chosen activation heights.

use zcash_protocol::consensus::{self, BlockHeight, NetworkType, NetworkUpgrade, Parameters};
use zcash_protocol::local_consensus::LocalNetwork;

#[derive(Clone, Copy, Debug)]
pub enum Net {
    Main,
    Test,
    Regtest(LocalNetwork),
}

impl Net {
    /// Regtest where pre-NU5 upgrades activate at 1 and NU5 up to `latest` activate at `nu_height`.
    /// `latest` is one of "nu5", "nu6", "nu6_1", "nu6_2", "nu6_3", "nu7"; later upgrades stay inactive.
    pub fn regtest(nu_height: u32, latest: &str) -> Net {
        let order = ["nu5", "nu6", "nu6_1", "nu6_2", "nu6_3", "nu7"];
        let rank = order.iter().position(|n| *n == latest).unwrap_or(2);
        let h = |x: u32| Some(BlockHeight::from_u32(x));
        let at = |i: usize| if i <= rank { h(nu_height) } else { None };
        Net::Regtest(LocalNetwork {
            overwinter: h(1),
            sapling: h(1),
            blossom: h(1),
            heartwood: h(1),
            canopy: h(1),
            nu5: at(0),
            nu6: at(1),
            nu6_1: at(2),
            nu6_2: at(3),
            nu6_3: at(4),
            nu7: at(5),
        })
    }
}

impl Parameters for Net {
    fn network_type(&self) -> NetworkType {
        match self {
            Net::Main => consensus::Network::MainNetwork.network_type(),
            Net::Test => consensus::Network::TestNetwork.network_type(),
            Net::Regtest(l) => l.network_type(),
        }
    }

    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match self {
            Net::Main => consensus::Network::MainNetwork.activation_height(nu),
            Net::Test => consensus::Network::TestNetwork.activation_height(nu),
            Net::Regtest(l) => l.activation_height(nu),
        }
    }
}
