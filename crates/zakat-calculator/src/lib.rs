use std::time::{Duration, SystemTime};

/// One lunar year (354 days), per the Hanafi school adopted by AAOIFI FAS 9
/// Appendix D and SS-35 §3/2/4. Within a hawl year, intermediate dips below
/// nisab do not reset the clock — only the beginning and ending balances of
/// the year matter. The clock restarts from scratch when the balance falls
/// below nisab at an accrual date (chain break).
pub const HAWL_DURATION: Duration = Duration::from_secs(354 * 24 * 3600);

/// A running balance at a point in time. Feed chronologically-ordered slices
/// of these into [`assess_account`] or [`assess_all_periods`].
pub struct BalanceSnapshot {
    pub timestamp: SystemTime,
    /// Balance in the smallest currency unit (e.g. fils for AED).
    pub balance: i64,
}

/// The zakat obligation for one completed hawl period.
pub struct ZakatAssessment {
    /// When the hawl clock started (first time balance reached nisab).
    pub hawl_started_at: SystemTime,
    /// The accrual date: `hawl_started_at + n × HAWL_DURATION`.
    /// Per SS-35 §5/2/6/2–3, assets are valued "on the day of Zakah accrual".
    pub hawl_completed_at: SystemTime,
    /// Net zakatable balance in fils on the accrual date.
    ///
    /// For [`assess_account`] this equals the actual ledger balance.
    /// For [`assess_all_periods`] this is the ledger balance minus
    /// `prior_zakat_deducted`: the net base on which zakat is computed.
    pub balance: i64,
    /// Nisab threshold used, in fils.
    pub nisab: i64,
    /// Amount due (2.5% = 1/40 of `balance`), in fils.
    pub zakat_due: i64,
    /// Cumulative zakat from all earlier periods deducted before computing
    /// this period's obligation. Always zero in [`assess_account`] results.
    pub prior_zakat_deducted: i64,
}

// ── Engine ────────────────────────────────────────────────────────────────────

/// Walk `history` and collect every completed hawl period where zakat arose.
///
/// When `deduct_debt` is `true`, each period's `zakat_due` is accumulated as
/// an outstanding dayn (debt) and subtracted from the zakatable base of every
/// subsequent period before computing that period's obligation.  When `false`,
/// `accumulated_debt` stays zero throughout and `prior_zakat_deducted` is
/// always 0.
///
/// The balance lookup for each accrual date is O(n) — fine for typical account
/// histories but worth noting if histories grow very large.
fn collect_accruals(
    history: &[BalanceSnapshot],
    nisab_fils: i64,
    deduct_debt: bool,
) -> Vec<ZakatAssessment> {
    let now = SystemTime::now();
    let mut offset = 0;
    let mut accruals: Vec<ZakatAssessment> = Vec::new();
    // Stays zero when deduct_debt is false; grows with each accrual otherwise.
    let mut accumulated_debt: i64 = 0;

    while offset < history.len() {
        // Hawl starts when the net balance (actual minus prior debt) first
        // reaches nisab — consistent with the net assets method (SS-35 §2/1/1).
        let rel = history[offset..]
            .iter()
            .position(|s| s.balance.saturating_sub(accumulated_debt) >= nisab_fils);
        let Some(rel) = rel else { break };
        let hawl_start = history[offset + rel].timestamp;

        let mut n = 1u32;
        let mut broke_at: Option<SystemTime> = None;

        loop {
            let Some(hawl_end) = hawl_start.checked_add(HAWL_DURATION * n) else {
                break;
            };
            if hawl_end > now {
                break;
            }

            // Ledger balance at the accrual date (SS-35 §5/2/6/2–3).
            let actual_balance = history
                .iter()
                .filter(|s| s.timestamp <= hawl_end)
                .last()
                .map(|s| s.balance)
                .unwrap_or(0);

            let net_balance = actual_balance.saturating_sub(accumulated_debt);

            if net_balance < nisab_fils {
                // Net wealth fell below nisab at this accrual date — chain breaks.
                broke_at = Some(hawl_end);
                break;
            }

            let prior_zakat_deducted = accumulated_debt;
            let zakat_due = net_balance / 40;
            if deduct_debt {
                accumulated_debt = accumulated_debt.saturating_add(zakat_due);
            }

            accruals.push(ZakatAssessment {
                hawl_started_at: hawl_start,
                hawl_completed_at: hawl_end,
                balance: net_balance,
                nisab: nisab_fils,
                zakat_due,
                prior_zakat_deducted,
            });

            n += 1;
        }

        match broke_at {
            Some(t) => {
                offset = history
                    .iter()
                    .position(|s| s.timestamp > t)
                    .unwrap_or(history.len());
            }
            None => break,
        }
    }

    accruals
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Assess whether zakat is due on an account given its full balance history,
/// returning the **most recent** accrual where the obligation arose.
///
/// Makes **no assumption** about whether zakat was paid in prior years —
/// operates on actual ledger balances. Use for the "is zakat currently due?"
/// question. Use [`assess_all_periods`] for the "what is the total unpaid
/// amount?" question.
///
/// Returns `None` when no completed hawl with a nisab-meeting balance exists.
/// `history` must be chronologically ordered (oldest first).
pub fn assess_account(history: &[BalanceSnapshot], nisab_fils: i64) -> Option<ZakatAssessment> {
    collect_accruals(history, nisab_fils, false)
        .into_iter()
        .last()
}

/// Return **every** accrual period in which zakat was due, oldest first,
/// modelling the case where **no prior-year zakat was paid**.
///
/// Each period's zakatable base is the actual ledger balance on the accrual
/// date minus the cumulative unpaid zakat from all earlier periods
/// (`prior_zakat_deducted`). This prevents computing zakat on fils that
/// Shari'ah already required to be disbursed.
///
/// ## When to use
///
/// Call this when a customer wants to settle a zakat backlog and needs to know
/// the total outstanding across all years. If the customer paid zakat in some
/// years, present the full list and let them identify which obligations they
/// have already discharged before summing.
///
/// ## Scholarly basis and limitations
///
/// Treating prior-year unpaid zakat as a deductible dayn is the majority
/// classical fiqh position (Hanafi, Shafi'i, Hanbali): unpaid zakat is a
/// confirmed obligation on the wealth, and computing subsequent years' zakat
/// on a gross balance that already includes money that should have been
/// disbursed effectively taxes the same fils twice.
///
/// This is consistent with AAOIFI SS-35 §2/1/1 (net assets method: zakatable
/// assets minus qualifying liabilities) and §6/2/1 (debts arising from
/// obtaining zakatable assets are deductible). SS-35 §10/5 confirms that
/// "Zakah does not cease to be valid by prescription."
///
/// **Important**: SS-35 is an institutional standard and does not explicitly
/// address this scenario for individual depositors. Customers should consult a
/// qualified Islamic scholar (alim/mufti) to confirm the ruling applicable to
/// their situation.
///
/// `history` must be chronologically ordered (oldest first).
pub fn assess_all_periods(history: &[BalanceSnapshot], nisab_fils: i64) -> Vec<ZakatAssessment> {
    collect_accruals(history, nisab_fils, true)
}
