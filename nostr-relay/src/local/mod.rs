mod subscriptions;
mod serve;
mod hooks;

use std::{collections::HashMap, net::SocketAddr};

use nostr_surreal_db::message::{notice::Notice, subscription::Subscription};
use rand::random;
use tokio::sync::mpsc;

use crate::GlobalState;

/// A subscription identifier has a maximum length
const MAX_SUBSCRIPTION_ID_LEN: usize = 256;

/// LocalState represents the local state of a single websocket connection
pub struct LocalState {
    pub(crate) client_ip_addr: SocketAddr,
    pub(crate) subscriptions: HashMap<String, Subscription>,
    pub(crate) max_subs: usize,

    pub(crate) auth_challenge: [u8; 32],

    pub(crate) outgoing_sender: mpsc::Sender<Notice>,
    pub(crate) global_state: GlobalState,
    pub(crate) is_authenticated: bool,
}

// On subscriptions
impl LocalState {
    pub fn new(
        client_ip_addr: SocketAddr,
        outgoing_sender: mpsc::Sender<Notice>,
        global_state: GlobalState,
    ) -> Self {
        Self {
            client_ip_addr, subscriptions: HashMap::new(),
            max_subs: 32,
            auth_challenge: random(),
            outgoing_sender,
            global_state,
            is_authenticated: false,
        }
    }

    pub fn auth_challenge(&self) -> [u8; 32] {
        self.auth_challenge.clone()
    }
}