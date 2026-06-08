//! Reference implementation of the event consumer pattern.
//!
//! When writing a crate that needs to react to ledger events:
//!
//! 1. Subscribe to the `EventBus` **before** wrapping it in `Arc`:
//!    ```ignore
//!    let mut bus = EventBus::new();
//!    let rx = bus.subscribe(128);      // choose a buffer that fits your throughput
//!    let bus = Arc::new(bus);
//!    ```
//!
//! 2. Pass the receiver to your consumer and spawn it:
//!    ```ignore
//!    EventConsumer::new(rx).spawn();
//!    ```
//!
//! 3. The loop runs until all `EventBus` senders are dropped (i.e. when the
//!    ledger service shuts down), at which point the thread exits cleanly.
//!
//! **Idempotency**: the outbox relay guarantees at-least-once delivery, so
//! consumers must handle duplicate events. The pattern shown here tracks a
//! per-account high-water mark (`last_journal_entry_id`) in memory. A
//! production consumer would persist this cursor to its own database so
//! replays after a restart are also handled correctly.

use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;

use domain_events::DomainEvent;

/// Receives `DomainEvent`s from the bus and logs them to stdout.
///
/// Serves as a working reference for the subscription and idempotency
/// patterns. Copy the structure, swap the handler body for your logic, and
/// replace the in-memory cursor with a persisted one.
pub struct EventConsumer {
    receiver: Receiver<DomainEvent>,
}

impl EventConsumer {
    pub fn new(receiver: Receiver<DomainEvent>) -> Self {
        Self { receiver }
    }

    pub fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            // In-memory idempotency cursor: account_id → last processed journal_entry_id.
            // A production consumer persists this to its own DB table so restarts
            // don't reprocess events that were already handled.
            let mut cursor: HashMap<i64, i64> = HashMap::new();

            for event in &self.receiver {
                match event {
                    DomainEvent::BalanceChanged {
                        account_id,
                        new_balance,
                        journal_entry_id,
                    } => {
                        // Skip if this event (or a later one) was already handled.
                        if cursor
                            .get(&account_id)
                            .is_some_and(|&last| journal_entry_id <= last)
                        {
                            continue;
                        }

                        // ── Your domain logic goes here ────────────────────
                        println!(
                            "[event-consumer-demo] account={account_id} \
                             balance={new_balance} journal_entry={journal_entry_id}"
                        );
                        // ──────────────────────────────────────────────────

                        cursor.insert(account_id, journal_entry_id);
                    }
                }
            }
        })
    }
}
