use kyoto::{Address, Txid};

// use crate::error::Error;
use bitcoin::Amount;
// use bitcoin::locktime::Height;

use std::collections::HashMap;

type Height = u32;

struct UtxoInfo {
    is_relevant: bool,
    // Store outputs, but only relevant outputs
    out: HashMap<Address, Amount>,
    // Height confirmed
    height: Height,
    // Height spend
    spent_height: Option<Height>,
}

impl UtxoInfo {
    fn new(height: Height, is_relevant: bool) -> Self {
        Self {
            is_relevant,
            out: HashMap::new(),
            height,
            spent_height: None,
        }
    }

    fn add_output(&mut self, addr: Address, value: Amount) {
        self.out.insert(addr, value);
        self.is_relevant = true;
    }

    fn total_value(&self) -> u64 {
        let mut s = 0u64;
        for (_addr, value) in &self.out {
            s = s + value.to_sat();
        }
        s
    }

    fn set_spent(&mut self, spent_height: Height) {
        self.spent_height = Some(spent_height);
    }

    fn is_spent(&self) -> bool {
        self.spent_height.is_some()
    }
}

pub(crate) struct FullBalance {
    pub inn: u64,
    pub out: u64,
}

impl FullBalance {
    pub fn current(&self) -> u64 {
        self.inn - self.out
    }
}

pub(crate) struct UtxoStore {
    utxos: HashMap<Txid, UtxoInfo>,
}

impl UtxoStore {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    pub fn balance_full(&self) -> FullBalance {
        let mut inn = 0u64;
        let mut out = 0u64;
        for (_txid, u) in &self.utxos {
            if u.is_relevant {
                let val = u.total_value();
                inn = inn + val;
                if u.is_spent() {
                    out = out + val;
                }
            }
        }
        FullBalance { inn, out }
    }

    // pub fn balance(&self) -> u64 { self.balance_full().current() }

    // Add a utxo with a relevant output
    pub fn add_utxo(
        &mut self,
        height: Height,
        txid: Txid,
        _vout: u32,
        address: Address,
        value: Amount,
    ) {
        if !self.utxos.contains_key(&txid) {
            self.utxos.insert(txid, UtxoInfo::new(height, true));
        }
        self.utxos
            .get_mut(&txid)
            .unwrap()
            .add_output(address, value);
        println!("UtxoStore size {}", self.utxos.len());
    }

    pub fn set_utxo_spent(&mut self, height: Height, txid: Txid) {
        if !self.utxos.contains_key(&txid) {
            self.utxos.insert(txid, UtxoInfo::new(height, false)); // we don't know if this is relevant, only that it's spent
        }
        self.utxos.get_mut(&txid).unwrap().set_spent(height);
        // println!("UtxoStore size {}", self.utxos.len());
    }

    pub fn get_txo_counts(&self) -> (usize, usize) {
        let (mut uc, mut sc) = (0, 0);
        for (_txid, u) in &self.utxos {
            if u.is_relevant {
                if u.is_spent() {
                    sc = sc + 1;
                } else {
                    uc = uc + 1;
                }
            }
        }
        (uc, sc)
    }
}
