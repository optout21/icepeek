use icepeek_app::app::{AppAsync, AppStateUpdate};

fn callback(app: &AppStateUpdate) {
    let state = app.state();
    println!("{:?}", state);
    print!("Balance: {}", state.balance);
    if state.balance != 0 || state.utxo_count != 0 || state.stxo_count != 0 {
        print!("  ({}/{})", state.balance_in, state.balance_out);
        print!(
            "  TXOs: {} (unspent, spent {})",
            state.utxo_count, state.stxo_count
        );
    }
    println!("");
    print!("Tips: ");
    if state.header_tip != 0 {
        print!(
            " scan {:.1}% ({})    filter {:.1}% ({})",
            state.get_scan_tip_pct(),
            state.scan_tip,
            state.get_filter_tip_pct(),
            state.filter_tip
        );
    }
    println!("  header {}", state.header_tip);
}

#[tokio::main]
async fn main() {
    println!("IcePeek CLI");

    // Add third-party logging
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let mut wallet_definition = AppStateUpdate::prepare_sample_wallet_definition();
    wallet_definition.address_count_initial = 10;

    {
        println!("Watched addresses:");
        if let Ok(addrs) = AppStateUpdate::derive_addresses(&wallet_definition) {
            for a in &addrs {
                println!(
                    "  {}  ({})",
                    a.address.to_string(),
                    &a.derivation.to_string()
                );
            }
        }
    }

    let mut app = AppAsync::create_and_start(wallet_definition, callback).expect("Create AppAsync");
    app.stop().await;
}
