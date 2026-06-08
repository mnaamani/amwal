use std::collections::HashMap;

use ledger::service::LedgerService;
use ledger_api::LedgerClient;
use zakat_calculator::{BalanceSnapshot, ZakatAssessment, assess_account, assess_all_periods};

// ── Nisab configuration ───────────────────────────────────────────────────────

/// The school of Islamic jurisprudence — determines nisab weight threshold and
/// debt-deduction rules.
#[derive(Debug, Clone, Copy)]
enum Madhab {
    Hanafi,
    Maliki,
    Shafii,
    Hanbali,
}

impl Madhab {
    fn api_key(self) -> &'static str {
        match self {
            Madhab::Hanafi => "hanafi",
            Madhab::Maliki => "maliki",
            Madhab::Shafii => "shafii",
            Madhab::Hanbali => "hanbali",
        }
    }
}

/// Whether to use the gold or silver nisab threshold.
///
/// Silver is numerically lower (~AED 1,500 vs ~AED 12,000 at current prices).
/// Most scholars recommend gold for currency and cash savings.
#[derive(Debug, Clone, Copy)]
enum NisabStandard {
    Gold,
    Silver,
}

// Internal serde types for the tahababa API response.
#[derive(serde::Deserialize)]
struct ApiResponse {
    nisab: HashMap<String, MadhabNisab>,
}

#[derive(serde::Deserialize)]
struct MadhabNisab {
    gold: NisabValues,
    silver: NisabValues,
}

#[derive(serde::Deserialize)]
struct NisabValues {
    values: HashMap<String, f64>,
}

/// Fetch the current nisab threshold from nisab.tahababa.com in AED minor units.
///
/// The API publishes live gold/silver spot prices for 37 currencies, updated
/// up to 6× per day. Values are in the major currency unit; we multiply by 100
/// to convert to minor units to match the ledger's integer money representation.
fn fetch_nisab(madhab: Madhab, standard: NisabStandard) -> anyhow::Result<i64> {
    let resp: ApiResponse = ureq::get("https://nisab.tahababa.com/nisab.json")
        .call()?
        .into_json()?;

    let madhab_data = resp
        .nisab
        .get(madhab.api_key())
        .ok_or_else(|| anyhow::anyhow!("madhab '{}' not in API response", madhab.api_key()))?;

    let standard_values = match standard {
        NisabStandard::Gold => &madhab_data.gold,
        NisabStandard::Silver => &madhab_data.silver,
    };

    let aed = standard_values
        .values
        .get("AED")
        .ok_or_else(|| anyhow::anyhow!("AED not found in nisab values"))?;

    Ok((aed * 100.0).round() as i64)
}

// ── Report data ───────────────────────────────────────────────────────────────

struct AccountData {
    id: i64,
    name: String,
    history: Vec<BalanceSnapshot>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fmt_aed(amount: i64) -> String {
    format!("AED {:.2}", amount as f64 / 100.0)
}

fn fmt_date(t: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    dt.format("%Y-%m-%d").to_string()
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    // Defaults — could be made into CLI args or env vars later.
    let madhab = Madhab::Hanafi;
    let standard = NisabStandard::Gold;

    println!("Fetching nisab ({madhab:?} / {standard:?})...");
    let nisab = fetch_nisab(madhab, standard)?;
    println!("Nisab: {} ({nisab} minor units)\n", fmt_aed(nisab));

    let ledger = LedgerService::from_env();
    let accounts = ledger
        .list_active_accounts()
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    println!("Scanning {} active accounts...\n", accounts.len());

    // Load every account's balance history once; both reports share the same data.
    let mut account_data: Vec<AccountData> = Vec::new();
    for account in accounts {
        let history = ledger
            .get_balance_history(account.id)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?
            .into_iter()
            .map(|s| BalanceSnapshot {
                timestamp: s.timestamp,
                balance: s.balance,
            })
            .collect();
        account_data.push(AccountData {
            id: account.id,
            name: account.name,
            history,
        });
    }

    // ── Current-status report ─────────────────────────────────────────────────
    println!("══════════════════════════════════════════════");
    println!(" CURRENT ZAKAT STATUS");
    println!("══════════════════════════════════════════════\n");

    let mut due_count = 0usize;
    for ad in &account_data {
        let Some(a) = assess_account(&ad.history, nisab) else {
            continue;
        };
        due_count += 1;
        println!(
            "Account #{} — {}\n  Hawl started  : {}\n  Accrual date  : {}\n  Balance       : {}\n  Nisab         : {}\n  Zakat due     : {}\n",
            ad.id,
            ad.name,
            fmt_date(a.hawl_started_at),
            fmt_date(a.hawl_completed_at),
            fmt_aed(a.balance),
            fmt_aed(a.nisab),
            fmt_aed(a.zakat_due),
        );
    }

    if due_count == 0 {
        println!("No accounts have zakat due at this time.\n");
    } else {
        println!("{due_count} account(s) have zakat due.\n");
    }

    // ── Arrears report ────────────────────────────────────────────────────────
    println!("══════════════════════════════════════════════");
    println!(" ZAKAT ARREARS (ALL UNPAID PERIODS)");
    println!("══════════════════════════════════════════════\n");

    let mut grand_total: i64 = 0;
    let mut arrears_count = 0usize;

    for ad in &account_data {
        let periods: Vec<ZakatAssessment> = assess_all_periods(&ad.history, nisab);
        if periods.is_empty() {
            continue;
        }

        let account_total: i64 = periods.iter().map(|p| p.zakat_due).sum();
        grand_total += account_total;
        arrears_count += 1;

        println!("Account #{} — {}", ad.id, ad.name);
        for (i, p) in periods.iter().enumerate() {
            let deduction_note = if p.prior_zakat_deducted > 0 {
                format!("  less prior debt {}  →", fmt_aed(p.prior_zakat_deducted))
            } else {
                String::new()
            };
            println!(
                "  Year {:>2}  {}{}  net base {}  due {}",
                i + 1,
                fmt_date(p.hawl_completed_at),
                deduction_note,
                fmt_aed(p.balance),
                fmt_aed(p.zakat_due),
            );
        }
        println!("  ─────────────────────────────────────────");
        println!("  Account total : {}\n", fmt_aed(account_total));
    }

    if arrears_count == 0 {
        println!("No zakat accrual history found.");
    } else {
        println!("══════════════════════════════════════════════");
        println!(
            " GRAND TOTAL (all accounts, all years) : {}",
            fmt_aed(grand_total)
        );
        println!("══════════════════════════════════════════════");
        println!();
        println!("Note: each year's net base deducts prior unpaid zakat as a dayn");
        println!("(debt), per the majority classical fiqh position (Hanafi/Shafi'i/");
        println!("Hanbali). Consistent with SS-35 §6/2/1 and §10/5, but AAOIFI does");
        println!("not address this scenario for individual depositors explicitly.");
        println!("Consult a qualified Islamic scholar before settling.");
        println!();
        println!("This report assumes NO prior zakat was paid. Deduct any years");
        println!("already discharged before treating the total as outstanding.");
    }

    Ok(())
}
