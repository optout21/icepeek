// use nakamoto_client::handle;
use std::io;
use thiserror::Error;

// use crate::input;

/// An error occuring in the wallet.
#[derive(Error, Debug)]
pub enum Error {
    // #[error("client handle error: {0}")]
    // Handle(#[from] handle::Error),
    // #[error("client error: {0}")]
    // Client(#[from] nakamoto_client::Error),
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    // #[error("input error: {0}")]
    // Input(#[from] input::Error),
    // #[error("ui error: {0}")]
    // Ui(#[from] ui::Error),
    #[error("channel error: {0}")]
    Channel(#[from] crossbeam_channel::RecvError),
    // #[error(transparent)]
    // Db(#[from] db::Error),
    // #[error(transparent)]
    // Hw(#[from] hw::Error),
}
