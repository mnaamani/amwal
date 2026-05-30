use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use serde::{Deserialize, Serialize};

/// Events emitted after a state change. Each variant carries
/// enough data for downstream modules to act without querying back.
///
/// `Clone` is required so the bus can fan-out to multiple subscribers.
/// `Serialize`/`Deserialize` are needed for the transactional outbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    /// Emitted after any journal entry that changes an account's posted balance.
    BalanceChanged {
        /// Ledger account ID (i32, mirrors `ledger_api::AccountId`).
        account_id: i32,
        new_balance: i64,
        /// The journal entry that caused this change. Used by consumers as a
        /// high-water mark to skip already-processed events on replay.
        journal_entry_id: i32,
    },
}

/// In-process pub/sub bus backed by bounded `std::sync::mpsc` channels.
///
/// Set up subscribers **before** wrapping in `Arc` — `subscribe` requires
/// `&mut self`. Once frozen in an `Arc`, only `publish` is accessible.
///
/// To replace with an external broker (NATS, Kafka, etc.), implement the same
/// `publish` / `subscribe` contract on a new struct; the ledger and consumer
/// crates need no changes.
pub struct EventBus {
    senders: Vec<SyncSender<DomainEvent>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
        }
    }

    /// Register a subscriber. Returns the receive end; the send end is held
    /// by the bus for fan-out. `buffer` is the number of events that can queue
    /// before `publish` blocks.
    pub fn subscribe(&mut self, buffer: usize) -> Receiver<DomainEvent> {
        let (tx, rx) = sync_channel(buffer);
        self.senders.push(tx);
        rx
    }

    /// Broadcast an event to every subscriber. Dead receivers (subscriber
    /// thread exited) are silently ignored.
    pub fn publish(&self, event: DomainEvent) {
        for sender in &self.senders {
            let _ = sender.send(event.clone());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
