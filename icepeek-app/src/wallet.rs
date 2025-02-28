// use bip39::Mnemonic;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub}; // Xpriv
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Address;

use std::str::FromStr;

/// Description of an address
#[derive(Clone)]
pub struct AddressInfo {
    pub address: Address,
    /// The derivation corresponding to the address
    pub derivation: DerivationPath,
}

/// Info defining a wallet, XPub
#[derive(Clone)]
pub struct WalletDefinition {
    pub network: bitcoin::Network,
    /// The XPub, in string format
    pub xpub: String,
    /// The derivation path corresponding to the XPub, in string format
    pub derivation_path: String,
    /// The number of addresses to generate upfront
    pub address_count_initial: u16,
    /// Hint to the first relevant block, scan before this can be omitted. If unsure, leave it to 0
    pub birth_height_hint: u32,
}

/// A wallet consisting of a set of addresses
#[derive(Clone)]
pub struct Wallet {
    pub wallet_definition: WalletDefinition,
    /// The list of addresses within this wallet
    pub addrs: Vec<AddressInfo>,
}

impl Wallet {
    pub fn new(wallet_definition: WalletDefinition) -> Result<Self, String> {
        let addrs = Self::derive_addresses(&wallet_definition)?;
        Ok(Self {
            wallet_definition,
            addrs,
        })
    }

    pub fn address_count(&self) -> usize {
        self.addrs.len()
    }

    pub fn addrs(&self) -> Vec<AddressInfo> {
        self.addrs.iter().map(|a| a.clone()).collect()
    }

    /*
    fn generate_test_xpub() -> Result<Xpub, String> {
        let mns0 = "oil oil oil ...";
        let mn = Mnemonic::parse(mns0).map_err(|e| format!("Parsing mnemonic {}", e))?;
        let seed = mn.to_seed_normalized("");
        let xpriv = Xpriv::new_master(bitcoin::Network::Testnet, &seed)
            .map_err(|e| format!("Creating XPriv {}", e))?;
        let derivation_path_base = DerivationPath::from_str("m/84'/0'/0'")
            .map_err(|e| format!("Creating DerivationPath {}", e))?;
        let secp = Secp256k1::new();
        let xpriv_level_4 = xpriv
            .derive_priv(&secp, &derivation_path_base)
            .map_err(|e| format!("Derive level4 xpriv {}", e))?;
        let xpub_level_4 = Xpub::from_priv(&secp, &xpriv_level_4);
        Ok(xpub_level_4)
    }
    */

    pub fn derive_addresses(wallet_def: &WalletDefinition) -> Result<Vec<AddressInfo>, String> {
        let xpub0 = Xpub::from_str(&wallet_def.xpub).map_err(|e| format!("Parse XPub {}", e))?;
        // let xpub0 = Self::generate_test_xpub()?;
        // println!("Xpub {}", xpub0);
        if xpub0.network != bitcoin::NetworkKind::from(wallet_def.network) {
            return Err(format!(
                "Wrong network! {:?} {}",
                xpub0.network, wallet_def.network
            ));
        }
        let deriv0 = DerivationPath::from_str(&wallet_def.derivation_path)
            .map_err(|e| format!("Creating DerivationPath {}", e))?;
        let mut addresses = Vec::new();
        // 'Normal' (non-change) addresses
        Self::derive_addresses_intern(
            &xpub0,
            &deriv0,
            wallet_def.network,
            0,
            wallet_def.address_count_initial,
            &mut addresses,
        )?;
        // Change addresses
        Self::derive_addresses_intern(
            &xpub0,
            &deriv0,
            wallet_def.network,
            1,
            wallet_def.address_count_initial,
            &mut addresses,
        )?;
        Ok(addresses)
    }

    fn derive_addresses_intern(
        xpub: &Xpub,
        derivation: &DerivationPath,
        network: bitcoin::Network,
        index: u32,
        address_count: u16,
        addresses: &mut Vec<AddressInfo>,
    ) -> Result<(), String> {
        let secp = Secp256k1::new();
        let index_0 = ChildNumber::from_normal_idx(index).unwrap();
        let deriv_0 = derivation.child(index_0);
        let xpub_0 = xpub.derive_pub(&secp, &vec![index_0]).unwrap();
        let adjusted_count = std::cmp::max(address_count, 1);
        for i in 0..adjusted_count {
            let index_i = ChildNumber::from_normal_idx(i as u32).unwrap();
            let deriv_0_i = deriv_0.child(index_i);
            let pub_0_i = xpub_0
                .derive_pub(&secp, &vec![index_i])
                .map_err(|e| format!("Derive child pub {}", e))?
                .to_pub();
            // println!("{} {}", i, child_pub.to_string());
            let address = Address::p2wpkh(&pub_0_i, network);
            // .map_err(|e| format!("Derive address from pubkey {}", e))?;
            // println!("{} {}", i, addr.to_string());
            addresses.push(AddressInfo {
                address,
                derivation: deriv_0_i,
            });
        }
        Ok(())
    }
}
