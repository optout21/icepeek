use crate::error::Error;
use crate::logger;
use crate::smart_update::SmartUpdate;
use crate::utxo_store::UtxoStore;
use crate::wallet::{AddressInfo, Wallet, WalletDefinition};

use kyoto::{
    Address, BlockHash, ClientError, HeaderCheckpoint, Network, NodeBuilder, NodeMessage,
    ScriptBuf, Transaction,
};

use bitcoin::bip32::DerivationPath;
use tokio::task::JoinHandle;

use std::ops::ControlFlow;
use std::ops::ControlFlow::Break;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

type Height = u32;

/// The app state, data only
#[derive(Clone, Debug, Default)]
pub struct AppState {
    // Tips
    /// Block header
    pub header_tip: u64,
    /// Filter header
    pub filter_header_tip: u64,
    /// Filter
    pub filter_tip: u64,

    // Balance
    /// Current balance
    pub balance: u64,
    pub balance_in: u64,
    pub balance_out: u64,

    // TXOs
    /// Current numner of UTXOs
    pub utxo_count: u64,
    /// Current numner spent of TXOs (STXOs)
    pub stxo_count: u64,
}

impl PartialEq for AppState {
    fn eq(&self, other: &AppState) -> bool {
        self.header_tip == other.header_tip
            && self.filter_header_tip == other.filter_header_tip
            && self.filter_tip == other.filter_tip
            && self.balance == other.balance
            && self.balance_in == other.balance_in
            && self.balance_out == other.balance_out
    }
}

impl Eq for AppState {}

impl AppState {
    pub fn get_filter_header_tip_pct(&self) -> f64 {
        if self.header_tip == 0 {
            0f64
        } else {
            100f64 * (self.filter_header_tip as f64) / (self.header_tip as f64)
        }
    }

    pub fn get_filter_tip_pct(&self) -> f64 {
        if self.header_tip == 0 {
            0f64
        } else {
            100f64 * (self.filter_tip as f64) / (self.header_tip as f64)
        }
    }
}

pub struct Options {
    /// watch the following addresses
    // #[argh(option)]
    pub addresses: Vec<bitcoin::Address>,
    /// wallet birth height, from which to start scanning
    // #[argh(option)]
    pub birth_height: Height,
    /// network to connect to, eg. `testnet`
    // #[argh(option, default = "Network::default()")]
    pub network: Network,
    /// wallet derivation path, eg. m/84'/0'/0'/0.
    // #[argh(option)]
    pub hd_path: DerivationPath,
    /// enable debug logging
    // #[argh(switch)]
    pub debug: bool,
}

const DEFAULT_DERIVATION_PATH_BASE: &str = "m/84'/0'/0'";

// /// The network reactor we're going to use.
// type Reactor = nakamoto_net_poll::Reactor<net::TcpStream>;

type AppCallback = fn(app: &AppStateUpdate);

/// The application, with parameters, state and state update logic
pub struct AppStateUpdate {
    network: Network,
    state: AppState,
    callback: AppCallback,
    wallet: Wallet,
    utxo_store: UtxoStore,
    smart_update: SmartUpdate<AppState>,
}

impl AppStateUpdate {
    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub(crate) fn wallet(&self) -> Wallet {
        self.wallet.clone()
    }

    pub fn prepare_wallet_definition(
        network: &str,
        xpub: String,
        derivation_path: String,
        address_count_initial: String,
        birth_height_hint: String,
    ) -> WalletDefinition {
        let network = match network {
            "Mainnet" => bitcoin::Network::Bitcoin,
            _ => bitcoin::Network::Testnet,
        };
        WalletDefinition {
            network,
            xpub,
            derivation_path,
            address_count_initial: address_count_initial.parse().unwrap_or_default(),
            birth_height_hint: birth_height_hint.parse().unwrap_or_default(),
        }
    }

    pub fn prepare_sample_wallet_definition() -> WalletDefinition {
        Self::prepare_wallet_definition(
            "Mainnet",
            "xpub6CDDB17Xj7pDDWedpLsED1JbPPQmyuapHmAzQEEs2P57hciCjwQ3ov7TfGsTZftAM2gVdPzE55L6gUvHguwWjY82518zw1Z3VbDeWgx3Jqs".to_string(),
            DEFAULT_DERIVATION_PATH_BASE.to_string(),
            "20".to_string(),
            "640000".to_string(),
        )
    }

    pub fn derive_addresses(
        wallet_definition: &WalletDefinition,
    ) -> Result<Vec<AddressInfo>, String> {
        Wallet::derive_addresses(wallet_definition)
    }

    fn prepare_opts(wallet: &Wallet) -> Result<Options, String> {
        let wallet_definition = &wallet.wallet_definition;
        let opts = Options {
            addresses: wallet.addrs.iter().map(|a| a.address.clone()).collect(),
            birth_height: wallet_definition.birth_height_hint,
            network: wallet_definition.network,
            hd_path: DerivationPath::from_str(&wallet_definition.derivation_path).unwrap(),
            debug: false,
        };

        let level = if opts.debug {
            log::Level::Debug
        } else {
            log::Level::Warn // Error Warn Info
        };
        logger::init(level).expect("initializing logger for the first time");

        Ok(opts)
    }

    pub(crate) fn new(
        wallet_definition: WalletDefinition,
        app_callback: AppCallback,
    ) -> Result<(Self, Options), String> {
        println!("AppStateUpdate::new()");
        let wallet = Wallet::new(wallet_definition.clone())?;
        let opts = Self::prepare_opts(&wallet)?;

        let state = AppState::default();
        let app = Self {
            // network: Self::convert_network(&wallet_definition.network),
            network: wallet_definition.network,
            state: state.clone(),
            callback: app_callback,
            wallet,
            utxo_store: UtxoStore::new(),
            smart_update: SmartUpdate::new(250, state),
        };
        Ok((app, opts))
    }

    pub fn do_callback(&mut self, forced: bool) {
        let need_update = if forced {
            true
        } else {
            self.smart_update.update_state(self.state().clone())
        };
        if need_update {
            (self.callback)(&self);
        }
    }

    fn handle_client_event(
        &mut self,
        event: NodeMessage,
        watch: &[ScriptBuf],
    ) -> Result<ControlFlow<()>, Error> {
        // log::debug!("Received client event: {:?}, watch.len {}", event, watch.len());

        match event {
            NodeMessage::Dialog(d) => println!("Info: {}", d),
            NodeMessage::Warning(e) => {
                // TODO check Warnings
                println!("WARN: {}", e);
            }
            NodeMessage::StateChange(node_state) => println!("StateChange: {}", node_state),
            NodeMessage::ConnectionsMet => {
                println!("Peer connections met");
                self.do_callback(false);
            }
            NodeMessage::Progress(progress) => {
                self.state.header_tip = progress.tip_height as u64;
                self.state.filter_header_tip = progress.filter_headers as u64;
                self.state.filter_tip = progress.filters as u64;
                self.do_callback(false);
            }
            NodeMessage::Block(block) => {
                println!("Block: {}", block.height);
                for tx in &block.block.txdata {
                    self.apply(&tx, block.height, watch);
                }
                self.do_callback(true);
            }
            NodeMessage::Synced(update) => {
                let height = update.tip().height;
                // println!("Synced chain up to block {}", height);
                // println!("Chain tip: {}", update.tip().hash);
                self.state.header_tip = height as u64;
                // println!("Nakamoto BlockHeadersSynced {}", height)
                self.do_callback(false);
            }
            NodeMessage::BlocksDisconnected(_disconnected_headers) => (),
            NodeMessage::TxSent(_txid) => (),
            NodeMessage::TxBroadcastFailure(_failure_payload) => (),
        }
        Ok(ControlFlow::Continue(()))
    }

    /// Apply a transaction to the wallet's UTXO set.
    pub fn apply(&mut self, tx: &Transaction, height: u32, scripts: &[ScriptBuf]) {
        let txid = tx.compute_txid();

        // Print
        // println!("apply h {}  s.len {}  tx.out.n {}  txid {}  tx {:?}", height, scripts.len(), tx.output.len(), txid, &tx);
        for input in tx.input.iter() {
            // println!(" {} {}", input.previous_output.txid, input.previous_output.vout);
            self.utxo_store
                .set_utxo_spent(height, input.previous_output.txid);
        }

        // Look for outputs.
        for (vout, output) in tx.output.iter().enumerate() {
            // Received coin. Mark the address as *used*, and update the balance for that
            // address.
            if scripts.contains(&output.script_pubkey) {
                // println!("  Script contained in watched,  {}", output.script_pubkey.as_bytes().to_hex());
                // Update UTXOs.
                let addr = Address::from_script(&output.script_pubkey, self.network).unwrap();
                // addr_str = addr.to_string();

                println!("  tx {} {}  {} {}", txid, vout, addr, output.value);
                self.utxo_store
                    .add_utxo(height, txid, vout as u32, addr, output.value);
            }
            // println!("  {} {}  {} {}", txid, vout, addr_str, output.value);
        }

        let balance = self.utxo_store.balance_full();
        self.state.balance = balance.current();
        self.state.balance_in = balance.inn;
        self.state.balance_out = balance.out;
        let (utxo_count, stxo_count) = self.utxo_store.get_txo_counts();
        self.state.utxo_count = utxo_count as u64;
        self.state.stxo_count = stxo_count as u64;
        // println!("apply  post counts {} {}", utxo_count, stxo_count);
        self.do_callback(false);
    }
}

/// The application with background event handling
pub struct AppEventHandling {
    app: Arc<RwLock<AppStateUpdate>>,
    event_loop_thread: Option<JoinHandle<Result<(), Error>>>,
}

impl AppEventHandling {
    pub(crate) fn new(
        wallet_definition: WalletDefinition,
        app_callback: AppCallback,
        receiver: tokio::sync::broadcast::Receiver<NodeMessage>,
    ) -> Result<(Self, Options), String> {
        println!("AppEventHandling::new()");

        let (app, opts) = AppStateUpdate::new(wallet_definition, app_callback)?;
        let app = Arc::new(RwLock::new(app));

        // println!("Watched addresses:");
        // for a in &opts.addresses {
        //     println!("  {}", a.to_string());
        // }
        // println!("");
        let watch: Vec<_> = opts.addresses.iter().map(|a| a.script_pubkey()).collect();
        // println!("Watched output scripts:");
        // for w in &watch {
        //     println!("  {:?}", w.as_bytes().to_hex());
        // }
        // println!("");

        // // Split the client into components that send messages and listen to messages
        let mut app_event = Self {
            app: app.clone(),
            event_loop_thread: None,
        };

        // run event handling loop in the background
        let th1 =
            tokio::task::spawn(
                async move { Self::event_loop_blocking(&app, watch, receiver).await },
            );
        app_event.event_loop_thread = Some(th1);
        log::info!("AppEventHandling: background event handling loop started");

        Ok((app_event, opts))
    }

    pub async fn stop(&mut self) {
        if self.event_loop_thread.is_some() {
            let th = self.event_loop_thread.take().unwrap();
            let _res = th
                .await
                .unwrap()
                .map_err(|e| format!("Thread join, event loop {:?}", e));
        }
        self.event_loop_thread = None;
        log::info!("AppEventHandling: background event handling loop stopped");
    }

    pub fn state(&self) -> AppState {
        self.app.read().unwrap().state().clone()
    }

    pub fn wallet(&self) -> Wallet {
        self.app.read().unwrap().wallet()
    }

    pub fn do_callback(&mut self, forced: bool) {
        self.app.write().unwrap().do_callback(forced).clone()
    }

    /// Run the event processing loop
    pub async fn event_loop_blocking(
        app: &Arc<RwLock<AppStateUpdate>>,
        watch: Vec<ScriptBuf>,
        mut receiver: tokio::sync::broadcast::Receiver<NodeMessage>,
    ) -> Result<(), Error> {
        log::info!("AppEventHandling: waiting for events...");

        loop {
            if let Ok(message) = receiver.recv().await {
                if let Break(()) = app.write().unwrap().handle_client_event(message, &watch)? {
                    break;
                }
            }
        }
        log::info!("AppEventHandling: exiting loop");

        Ok(())
    }
}

/// The application with P2P client and asynchronous invocations
pub struct AppAsync {
    app: Arc<RwLock<AppEventHandling>>,
    wallet_ro: Wallet,
    thread_client: Option<JoinHandle<Result<(), ClientError>>>,
}

impl AppAsync {
    pub fn create_and_start(
        wallet_definition: WalletDefinition,
        app_callback: AppCallback,
    ) -> Result<AppAsync, String> {
        println!("AppAsync::create_and_start()");

        let wallet = Wallet::new(wallet_definition.clone())?;

        // Create a new kyoto node builder
        let builder = NodeBuilder::new(wallet_definition.network);
        let anchor_mainnet_640k = HeaderCheckpoint::new(
            640_000,
            BlockHash::from_str("0000000000000000000b3021a283b981dd08f4ccf318b684b214f995d102af43")
                .unwrap(),
        );
        /*
        let anchor_mainnet_710k = HeaderCheckpoint::new(
            710_000,
            BlockHash::from_str("00000000000000000007822e1ddba0bed6a55f0072aa1584c70a2f81c275f587")
                .unwrap(),
        );
        */
        // Add node preferences and build the node/client
        let (node, client) = builder
            // The Bitcoin scripts to monitor
            .add_scripts(
                wallet
                    .addrs()
                    .iter()
                    .map(|ai| ai.address.script_pubkey())
                    .collect(),
            )
            // Only scan blocks strictly after an anchor checkpoint
            .anchor_checkpoint(anchor_mainnet_640k)
            // The number of connections we would like to maintain
            .num_required_peers(3)
            .build_node()
            .unwrap();

        // Split the client into components that send messages and listen to messages
        let (_sender, receiver) = client.split();

        // Run the node and wait for the sync message;
        tokio::task::spawn(async move { node.run().await });
        // log::info!("Node started...");

        let (app, _opts) = AppEventHandling::new(
            wallet_definition.clone(),
            app_callback,
            // node,
            // client.clone(),
            receiver,
        )?;
        let addrs = app.app.read().unwrap().wallet().addrs();
        let app = Arc::new(RwLock::new(app));

        // let watch: Vec<_> = opts.addresses.iter().map(|a| a.script_pubkey()).collect();
        log::info!("AppAsync: watching for {} addresses", addrs.len());

        let _birth_height_hint = wallet_definition.birth_height_hint;

        let app_async = Self {
            app: app.clone(),
            wallet_ro: Wallet::new(wallet_definition)?,
            thread_client: None,
        };

        Ok(app_async)
    }

    pub async fn stop(&mut self) {
        if self.thread_client.is_some() {
            let _res = self
                .thread_client
                .take()
                .expect("client thread")
                // .join()
                .await
                .unwrap()
                .map_err(|e| format!("Thread join, client thread {:?}", e));
        }
        self.app.write().unwrap().stop().await;
    }

    pub fn state(&self) -> AppState {
        self.app.read().unwrap().state().clone()
    }

    pub fn wallet_definition(&self) -> &WalletDefinition {
        &self.wallet_ro.wallet_definition
    }

    pub fn wallet(&self) -> Wallet {
        self.app.read().unwrap().wallet()
    }

    pub fn do_callback(&mut self, forced: bool) {
        self.app.write().unwrap().do_callback(forced).clone()
    }
}
