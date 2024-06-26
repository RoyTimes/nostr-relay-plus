use crate::client_command::ClientCommand;
use crate::event::UnsignedEvent;
use crate::request::Request;
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use nostr_crypto::sender_signer::SenderSigner;
use nostr_crypto::signer::Signer;
use nostr_plus_common::relay_message::RelayMessage;
use nostr_plus_common::relay_ok::RelayOk;
use nostr_plus_common::sender::Sender as NostrSender;
use nostr_plus_common::types::Bytes32;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub struct Client {
    signer: SenderSigner,
    tx: Option<Sender<ClientCommand>>,
    // ToDo: keep track of subscriptions
}

impl Client {
    pub fn new(signer: SenderSigner) -> Self {
        Self { signer, tx: None }
    }

    pub async fn connect(&mut self, url: &str) -> Result<()> {
        self._connect(url, None).await
    }

    pub async fn connect_with_channel(&mut self, url: &str) -> Result<Receiver<RelayMessage>> {
        let (sender, receiver) = mpsc::channel(10);
        self._connect(url, Some(sender)).await.map(|_| receiver)
    }

    pub async fn _connect(&mut self, url: &str, sink: Option<Sender<RelayMessage>>) -> Result<()> {
        // If already connected, ignore. ToDo: handle this better
        if self.tx.is_some() {
            return Err(anyhow!("Already connected"));
        }

        let (sender, mut receiver): (Sender<ClientCommand>, Receiver<ClientCommand>) =
            mpsc::channel(32);

        let (socket, resp) = match connect_async(url).await {
            Ok((socket, response)) => (socket, response),
            Err(_) => return Err(anyhow!("Cannot connect to {}", url)),
        };
        println!("Connection established: {:?}", resp);

        let (mut write, mut read) = socket.split();

        let sender_clone = sender.clone();
        // Spawn listener
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(message)) => {
                        Self::handle_incoming_message(message, &sender_clone, sink.as_ref()).await
                    }
                    Ok(Message::Ping(_)) => {} // tungstenite handles Pong already
                    Ok(Message::Close(c)) => println!("Server closed connection: {:?}", c), // ToDo: do something
                    Ok(_) => println!("Unsupported message"),
                    Err(err) => println!("Do something about {}", err), // ToDo: handle error
                }
            }
        });

        // Spawn sender, reading commands from internal channel
        tokio::spawn(async move {
            // Keep track of ack requests
            let mut ack_table: HashMap<Bytes32, oneshot::Sender<RelayOk>> = HashMap::new();

            while let Some(message) = receiver.recv().await {
                match message {
                    ClientCommand::Req(req) => {
                        if write.send(Message::from(req.to_string())).await.is_err() {
                            eprintln!("Req: websocket error"); // ToDo: do something better
                            break;
                        }
                    }
                    ClientCommand::Event((event, tx)) => {
                        // Add request to ack table
                        let _ = &ack_table.insert(event.id(), tx); // ToDo: handle existing entries
                        if write.send(Message::from(event.to_string())).await.is_err() {
                            eprintln!("Event: websocket error"); // ToDo: do something better
                            break;
                        }
                    }
                    ClientCommand::Ack(ok_msg) => {
                        if let Some(tx) = ack_table.remove(&ok_msg.event_id) {
                            let _ = tx.send(ok_msg);
                        }
                    }
                }
            }
        });

        self.tx = Some(sender);

        Ok(())
    }

    async fn handle_incoming_message(
        msg: String,
        tx: &Sender<ClientCommand>,
        sink: Option<&Sender<RelayMessage>>,
    ) {
        match serde_json::from_str::<RelayMessage>(&msg) {
            Ok(incoming) => match incoming {
                RelayMessage::Ok(ok_msg) => {
                    println!("ACK: {}", serde_json::to_string(&ok_msg).unwrap());
                    let _ = tx.send(ClientCommand::Ack(ok_msg)).await;
                }
                relay_msg => {
                    match sink {
                        None => println!("Unhandled: {:?}", msg),
                        Some(sender) => {
                            if let Err(e) = sender.send(relay_msg).await {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
            },
            Err(err) => eprintln!("ERROR ({}) with msg: {}", err, msg),
        }
    }

    pub async fn publish(&self, event: UnsignedEvent) -> Result<()> {
        match &self.tx {
            Some(sender) => {
                let event = event.sign(&self.signer)?;

                // Prepare ACK channel
                let (tx, rx) = oneshot::channel::<RelayOk>();

                sender.send(ClientCommand::Event((event, tx))).await?;

                // Wait for ACK with timeout
                match timeout(Duration::from_secs(20), rx).await {
                    Ok(Ok(msg)) => {
                        if msg.accepted {
                            Ok(())
                        } else {
                            Err(anyhow!("rejected: {}", msg.message))
                        }
                    }
                    Ok(Err(err)) => Err(anyhow!("request failed: {}", err)),
                    Err(err) => Err(anyhow!("ACK timeout: {}", err)),
                }
            }
            None => return Err(anyhow!("Publish: missing internal channel")),
        }
    }

    pub fn sender(&self) -> NostrSender {
        match &self.signer {
            SenderSigner::Schnorr(signer) => {
                NostrSender::SchnorrPubKey(signer.private.verifying_key().to_bytes().into())
            }
            SenderSigner::Eoa(signer) => NostrSender::EoaAddress(signer.address()),
        }
    }

    pub fn try_sign(&self, msg: &[u8; 32]) -> Result<Vec<u8>> {
        match &self.signer {
            SenderSigner::Schnorr(signer) => signer.try_sign(msg),
            SenderSigner::Eoa(signer) => signer.try_sign(msg),
        }
    }

    pub async fn subscribe(&self, req: Request) -> Result<()> {
        match &self.tx {
            Some(sender) => {
                sender.send(ClientCommand::Req(req)).await?;
            }
            None => return Err(anyhow!("Subscribe: missing websocket")),
        }
        Ok(())
    }
}
