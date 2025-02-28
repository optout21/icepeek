use icepeek_app::app::{AppAsync, AppStateUpdate};

use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{executor, subscription, Alignment, Application, Command, Element, Subscription, Theme};

use crossbeam::channel;
use once_cell::sync::Lazy;

/// Events that can affect the UI
#[derive(Clone, Debug)]
pub enum Event {
    AppStateChanged,
}

/// Used to notify the UI from the background workers
pub(crate) struct EventQueue {
    sender: channel::Sender<Event>,
    receiver: channel::Receiver<Event>,
}

impl EventQueue {
    fn new() -> Self {
        let (sender, receiver) = channel::bounded::<Event>(100);
        Self { sender, receiver }
    }

    pub fn push(&self, e: Event) -> Result<(), String> {
        self.sender
            .send(e)
            .map_err(|e| format!("InternalEventQueueSend {:?}", e))
    }

    pub fn pop(&self) -> Result<Event, String> {
        let e = self
            .receiver
            .recv()
            .map_err(|e| format!("InternalEventQueueRecv {:?}", e))?;
        Ok(e)
    }
}

/// Static event queue used to send notifications to the UI
static EVENT_QUEUE: Lazy<EventQueue> = Lazy::new(|| EventQueue::new());

#[derive(Debug, Clone)]
pub enum Message {
    Event(Event),
    Refresh,
    SetupComplete,
    SetupXpubChanged(String),
    SetupDerivationChanged(String),
    SetupCountChanged(String),
    SetupBirthChanged(String),
}

enum UiView {
    Setup,
    Main,
}

/// Settings passed through Iced to the app
pub(crate) struct AppSettings {
    pub network: &'static str,
}

impl AppSettings {
    fn network_values() -> Vec<&'static str> {
        vec!["Mainnet", "Testnet", "Signet"]
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            network: Self::network_values()[0],
        }
    }
}

pub(crate) struct IcedApp {
    view: UiView,
    app: Option<AppAsync>,
    setup_network: &'static str,
    setup_xpub: String,
    setup_derivation: String,
    setup_addr_count: String,
    setup_birth_hint: String,
    address_preview: String,
    utxos: String,
    utxo_last_serial: u32,
}

impl IcedApp {
    fn create_and_start(&mut self) {
        let wallet_definition = AppStateUpdate::prepare_wallet_definition(
            &self.setup_network,
            self.setup_xpub.clone(),
            self.setup_derivation.clone(),
            self.setup_addr_count.clone(),
            self.setup_birth_hint.clone(),
        );
        let app = AppAsync::create_and_start(wallet_definition, IcedApp::callback).unwrap();
        self.app = Some(app);
    }

    fn callback(app: &AppStateUpdate) {
        // println!("callback");
        let state = app.state();
        println!("{:?}", state);

        // UI notification
        let _res = EVENT_QUEUE.push(Event::AppStateChanged);
    }

    fn update_address_preview(&mut self) {
        self.address_preview = "...".to_owned();
        let wallet_definition = AppStateUpdate::prepare_wallet_definition(
            &self.setup_network,
            self.setup_xpub.clone(),
            self.setup_derivation.clone(),
            self.setup_addr_count.clone(),
            self.setup_birth_hint.clone(),
        );
        if let Ok(addrs) = AppStateUpdate::derive_addresses(&wallet_definition) {
            let mut s = "".to_string();
            for a in &addrs {
                s = s + &a.address.to_string() + " (" + &a.derivation.to_string() + ")\n";
            }
            self.address_preview = s;
        }
    }

    fn update_utxos(&mut self, force: bool) {
        if let Some(app) = self.app.as_ref() {
            let utxo_store = &app.utxo_store();
            let serial = utxo_store.serial_no();
            if force || self.utxo_last_serial == 0 || serial > self.utxo_last_serial {
                // need refresh
                let mut text = String::new();
                for (txid, ui) in utxo_store.get_utxos() {
                    if ui.is_relevant() {
                        for (addr, amnt) in ui.outputs() {
                            let txstr = txid.to_string();
                            text = format!(
                                "{} sats,  bl: {}  {}  {}  tx: {}..{}\n",
                                amnt.to_sat(),
                                ui.height(),
                                if let Some(spent) = ui.spent_height() {
                                    format!("spent: {}", spent)
                                } else {
                                    "(unspent)".to_string()
                                },
                                addr,
                                &txstr[0..6],
                                &txstr[txstr.len() - 4..txstr.len()],
                            ) + &text;
                        }
                    }
                }
                if text.len() == 0 {
                    text = "No UTXOs found".to_string();
                }
                self.utxos = text;
                self.utxo_last_serial = serial;
            }
        } else {
            self.utxos = "No UTXOs (init)".to_string();
            self.utxo_last_serial = 0;
        }
    }

    fn view_main(&self) -> Element<Message> {
        let app = self.app.as_ref().unwrap();
        let state = app.state();
        let wallet_definition = app.wallet_definition();
        let wallet = app.wallet();
        column![
            row![
                text("Balance: ").size(20),
                text(format!("{}", state.balance)).size(20),
            ]
            .padding(10),
            // UTXO and balance details
            row![
                text("Balance in/out: ").size(15),
                text(format!("{}/{}  ", state.balance_in, state.balance_out)).size(15),
                text("Transactions unspent/spent: ").size(15),
                text(format!("{}/{}  ", state.utxo_count, state.stxo_count)).size(15),
            ]
            .padding(10),
            // Wallet:
            row![
                text("Wallet addresses:  ").size(15),
                text(format!("{}", wallet.address_count())).size(15),
            ]
            .padding(10),
            // Wallet definition:
            row![
                text("XPub:  ").size(15),
                text(format!(
                    "{:.30}.. {} {}",
                    wallet_definition.xpub,
                    wallet_definition.derivation_path,
                    wallet_definition.address_count_initial
                ))
                .size(15),
            ]
            .padding(10),
            // P2P Network:
            row![
                text("Header tip: ").size(15),
                text(format!("{}  ", state.header_tip)).size(15),
                text("Filter header tip: ").size(15),
                text(format!(
                    "{:.1}% ({})  ",
                    state.get_filter_header_tip_pct(),
                    state.filter_header_tip
                ))
                .size(15),
                text("Filter tip: ").size(15),
                text(format!(
                    "{:.1}% ({})  ",
                    state.get_filter_tip_pct(),
                    state.filter_tip
                ))
                .size(15),
            ]
            .padding(10),
            row![text("UTXOs:").size(15),].padding(10),
            row![scrollable(text(&self.utxos).size(15)).height(200)].padding(10),
            row![button("(Refresh)").on_press(Message::Refresh),].padding(10),
        ]
        .padding(10)
        .align_items(Alignment::Start)
        .into()
    }

    fn view_setup(&self) -> Element<Message> {
        column![
            row![
                text("Enter the wallet definition").size(20),
            ]
            .padding(10),
            column![
                row![
                    text("Network: ").size(15),
                    text(&self.setup_network).size(15),
                    // pick_list(Self::network_values(), Some(&self.setup_network), |v| Message::SetupNetworkChanged(v)),
                ]
                .padding(2),
                row![
                    text("The wallet XPub, e.g. xpub6CDDB17Xj7pDDWedpLsED1JbPPQmyuapHmAzQEEs2P57hciCjwQ3ov7TfGsTZftAM2gVdPzE55L6gUvHguwWjY82518zw1Z3VbDeWgx3Jqs").size(15),
                ]
                .padding(2),
                row![
                    text_input("xpub...", &self.setup_xpub)
                    .on_input(Message::SetupXpubChanged).size(15),
                ]
                .padding(2),
                row![
                    text("The used derivation path, e.g. 'm/84'/0'/0'").size(15),
                ]
                .padding(2),
                row![
                    text_input("derivation...", &self.setup_derivation)
                    .on_input(Message::SetupDerivationChanged).size(15),
                ]
                .padding(2),
                row![
                    text("The number of addresses to generate, e.g. 20").size(15),
                ]
                .padding(2),
                row![
                    text_input("20", &self.setup_addr_count)
                    .on_input(Message::SetupCountChanged).size(15),
                ]
                .padding(2),
                row![
                    text("The optional birth height hint, ignore blocks before this. If unsure, leave it to 0").size(15),
                ]
                .padding(2),
                row![
                    text_input("    0", &self.setup_birth_hint)
                    .on_input(Message::SetupBirthChanged).size(15),
                ]
                .padding(2),
            ]
            .padding(10),
            column![
                row![
                    text("Address preview:")
                    .size(15),
                ]
                .padding(10),
                row![
                    scrollable(
                        text(&self.address_preview)
                        .size(15)
                    )
                    .height(200)
                ]
                .padding(10),
            ]
            .padding(10),
            row![
                button("Continue").on_press(Message::SetupComplete),
            ]
            .padding(10),
    ]
        .padding(10)
        .align_items(Alignment::Start)
        .into()
    }
}

pub enum SubscriptionState {
    Uninited,
    Inited,
}

impl Application for IcedApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = AppSettings;

    fn new(flags: AppSettings) -> (Self, Command<Message>) {
        let mut app = IcedApp {
            view: UiView::Setup,
            app: None,
            setup_network: &flags.network,
            setup_xpub: "xpub6CDDB17Xj7pDDWedpLsED1JbPPQmyuapHmAzQEEs2P57hciCjwQ3ov7TfGsTZftAM2gVdPzE55L6gUvHguwWjY82518zw1Z3VbDeWgx3Jqs".to_owned(), // TODO change to empty
            setup_derivation: "m/84'/0'/0'".to_owned(),
            setup_addr_count: "20".to_owned(),
            setup_birth_hint: "0".to_owned(),
            address_preview: "".to_owned(),
            utxos: "".to_owned(),
            utxo_last_serial: 0,
        };
        app.update_address_preview();
        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("IcePeek")
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::unfold(
            std::any::TypeId::of::<IcedApp>(),
            SubscriptionState::Uninited,
            move |state| async move {
                match state {
                    SubscriptionState::Uninited => (Message::Refresh, SubscriptionState::Inited),
                    SubscriptionState::Inited => match EVENT_QUEUE.pop() {
                        Err(e) => {
                            println!("DEBUG: Subscription: error {:?}", e);
                            (Message::Refresh, SubscriptionState::Inited)
                        }
                        Ok(event) => {
                            // println!("DEBUG: Subscription: Got event {:?}", event);
                            (Message::Event(event), SubscriptionState::Inited)
                        }
                    },
                }
            },
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Refresh => {
                self.update_utxos(true);
            }
            Message::Event(ev) => match ev {
                Event::AppStateChanged => {
                    self.update_utxos(false);
                }
            },
            Message::SetupComplete => {
                self.create_and_start();
                self.view = UiView::Main;
            }
            Message::SetupXpubChanged(v) => {
                self.setup_xpub = v;
                self.update_address_preview();
            }
            Message::SetupDerivationChanged(v) => {
                self.setup_derivation = v;
                self.update_address_preview();
            }
            Message::SetupCountChanged(v) => {
                self.setup_addr_count = v;
                self.update_address_preview();
            }
            Message::SetupBirthChanged(v) => {
                self.setup_birth_hint = v;
                // no need to update addresses
            } // Message::SetupNetworkChanged(v) => {
              //     self.setup_network = v;
              //     self.update_address_preview();
              // }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        match self.view {
            UiView::Main => self.view_main(),
            UiView::Setup => self.view_setup(),
        }
    }
}
