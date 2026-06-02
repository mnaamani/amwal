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
    /// The accrual date for this period: `hawl_started_at + n × HAWL_DURATION`.
    /// Per SS-35 §5/2/6/2–3, assets are valued "on the day of Zakah accrual".
    pub hawl_completed_at: SystemTime,
    /// Net zakatable balance in fils on the accrual date.
    ///
    /// For [`assess_account`] this is the actual ledger balance — no prior
    /// debt is assumed. For [`assess_all_periods`] this is the ledger balance
    /// minus `prior_zakat_deducted`: the net base on which zakat is computed.
    pub balance: i64,
    /// Nisab threshold used, in fils.
    pub nisab: i64,
    /// Amount due (2.5% = 1/40 of `balance`), in fils.
    pub zakat_due: i64,
    /// Cumulative zakat from all earlier periods deducted before computing
    /// this period's obligation. Zero when returned by [`assess_account`].
    ///
    /// Non-zero only in [`assess_all_periods`], which models the majority
    /// classical fiqh position (Hanafi, Shafi'i, Hanbali) that each
    /// prior year's unpaid zakat is a dayn (debt) reducing the net
    /// zakatable base of every subsequent period — consistent with the
    /// spirit of SS-35 §6/2/1 (debts reduce the Zakah base) and the net
    /// assets method of SS-35 §2/1/1, though AAOIFI does not address this
    /// specific scenario for individual depositors explicitly.
    ///
    /// Customers should verify this treatment with a qualified Islamic scholar.
    pub prior_zakat_deducted: i64,
}

// ── Inner engines ─────────────────────────────────────────────────────────────

/// Walk the full balance history and collect every accrual where zakat arose,
/// without applying any debt deduction between periods.
///
/// Used by [`assess_account`], which makes no assumption about whether prior
/// years' zakat was paid.
fn collect_all_accruals(history: &[BalanceSnapshot], nisab_fils: i64) -> Vec<ZakatAssessment> {
    let now = SystemTime::now();
    let mut offset = 0;
    let mut accruals: Vec<ZakatAssessment> = Vec::new();

    while offset < history.len() {
        let rel = history[offset..]
            .iter()
            .position(|s| s.balance >= nisab_fils);
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

            let accrual_balance = history
                .iter()
                .filter(|s| s.timestamp <= hawl_end)
                .last()
                .map(|s| s.balance)
                .unwrap_or(0);

            if accrual_balance < nisab_fils {
                broke_at = Some(hawl_end);
                break;
            }

            accruals.push(ZakatAssessment {
                hawl_started_at: hawl_start,
                hawl_completed_at: hawl_end,
                balance: accrual_balance,
                nisab: nisab_fils,
                zakat_due: accrual_balance / 40,
                prior_zakat_deducted: 0,
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

/// Walk the full balance history and collect every accrual where zakat arose,
/// applying the dayn-deduction model: each completed period's zakat is treated
/// as an outstanding debt that reduces the zakatable base of all subsequent
/// periods.
///
/// ## Scholarly basis
///
/// The dayn (debt) deduction principle is established in AAOIFI SS-35 §6/2/1:
/// debts that arise from obtaining zakatable assets are deductible from the
/// Zakah base, and §2/1/1 defines the net assets method as zakatable assets
/// minus qualifying liabilities. SS-35 §10/5 confirms that "Zakah does not
/// cease to be valid by prescription."
///
/// Treating prior-year **unpaid zakat** as a deductible dayn is the majority
/// classical fiqh position across the four schools, on the grounds that:
/// (a) unpaid zakat is a confirmed religious obligation (dayn) on the wealth,
/// (b) computing subsequent years' zakat on a gross balance that already
///     includes money that should have been disbursed effectively taxes the
///     same fils twice.
///
/// **Important**: AAOIFI SS-35 is an institutional standard and does not
/// explicitly address this scenario for individual depositors. The debt-
/// deduction model is consistent with the spirit of SS-35 and supported by
/// classical fiqh, but customers should consult a qualified Islamic scholar
/// (alim/mufti) to confirm the ruling applicable to their situation.
fn collect_all_accruals_debt_adjusted(
    history: &[BalanceSnapshot],
    nisab_fils: i64,
) -> Vec<ZakatAssessment> {
    let now = SystemTime::now();
    let mut offset = 0;
    let mut accruals: Vec<ZakatAssessment> = Vec::new();
    // Running total of unpaid zakat from all prior accrual periods.
    let mut accumulated_debt: i64 = 0;

    while offset < history.len() {
        // Hawl starts when the *net* balance (actual minus prior debt) first
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

            // Net zakatable base after deducting the accumulated prior debt.
            let net_balance = actual_balance.saturating_sub(accumulated_debt);

            if net_balance < nisab_fils {
                // Chain breaks: net wealth fell below nisab at this accrual date.
                broke_at = Some(hawl_end);
                break;
            }

            let prior_debt_this_period = accumulated_debt;
            let zakat_due = net_balance / 40;
            accumulated_debt = accumulated_debt.saturating_add(zakat_due);

            accruals.push(ZakatAssessment {
                hawl_started_at: hawl_start,
                hawl_completed_at: hawl_end,
                balance: net_balance,
                nisab: nisab_fils,
                zakat_due,
                prior_zakat_deducted: prior_debt_this_period,
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
/// This function makes **no assumption** about whether zakat was paid in prior
/// years — it operates on actual ledger balances. Use it for the "is zakat
/// currently due?" question. Use [`assess_all_periods`] for the "what is the
/// total unpaid amount?" question.
///
/// Returns `None` when no completed hawl with a nisab-meeting balance exists.
///
/// `history` must be in chronological order (oldest first).
/// `nisab_fils` is the threshold in the smallest currency unit.
pub fn assess_account(history: &[BalanceSnapshot], nisab_fils: i64) -> Option<ZakatAssessment> {
    collect_all_accruals(history, nisab_fils).into_iter().last()
}

/// Return **every** accrual period in which zakat was due, oldest first,
/// modelling the case where **no prior-year zakat was paid**.
///
/// Each period's zakatable base is the actual ledger balance on the accrual
/// date minus the cumulative unpaid zakat from all earlier periods
/// (`prior_zakat_deducted`). This prevents double-taxing fils that Shari'ah
/// already required to be disbursed.
///
/// ## When to use this function
///
/// Call this when a customer wants to settle a zakat backlog and needs to know
/// the total outstanding across all years. If the customer paid zakat in some
/// years, filter out those periods before summing or present the full list and
/// let the customer identify which obligations they have already discharged.
///
/// ## Scholarly basis and limitations
///
/// See [`collect_all_accruals_debt_adjusted`] for the full scholarly note.
/// In summary: the debt-deduction model is consistent with SS-35 §6/2/1 and
/// the majority classical fiqh position, but AAOIFI does not explicitly
/// address this scenario for individual depositors. Customers should confirm
/// the applicable ruling with a qualified Islamic scholar.
///
/// `history` must be in chronological order (oldest first).
/// `nisab_fils` is the threshold in the smallest currency unit.
pub fn assess_all_periods(history: &[BalanceSnapshot], nisab_fils: i64) -> Vec<ZakatAssessment> {
    collect_all_accruals_debt_adjusted(history, nisab_fils)
}
