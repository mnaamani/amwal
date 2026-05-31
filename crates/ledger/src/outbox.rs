use std::sync::Arc;
use std::thread;
use std::time::Duration;

use domain_events::{DomainEvent, EventBus};

use crate::postgres::PostgresLedgerStore;

/// Polls the outbox table and publishes deserialized events to the in-process
/// [`EventBus`]. Runs on its own thread — call [`OutboxRelay::spawn`] at
/// startup, after all bus subscribers have been registered.
///
/// Delivery is at-least-once: if the process crashes between `publish` and
/// `mark_outbox_delivered`, the event will be re-published on restart.
/// Consumers must be idempotent.
///
/// Unknown event types (e.g. emitted by a newer code version before this relay
/// was updated) are silently skipped and marked delivered so they do not block
/// the queue.
pub struct OutboxRelay {
    store: PostgresLedgerStore,
    bus: Arc<EventBus>,
    poll_interval: Duration,
}

impl OutboxRelay {
    pub fn new(store: PostgresLedgerStore, bus: Arc<EventBus>, poll_interval: Duration) -> Self {
        Self {
            store,
            bus,
            poll_interval,
        }
    }

    pub fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            loop {
                if let Ok(items) = self.store.fetch_pending_outbox(100) {
                    for item in items {
                        match serde_json::from_value::<DomainEvent>(item.payload) {
                            Ok(event) => self.bus.publish(event),
                            Err(_) => {} // unknown variant — fall through to mark delivered
                        }
                        let _ = self.store.mark_outbox_delivered(item.id);
                    }
                }
                thread::sleep(self.poll_interval);
            }
        })
    }
}
