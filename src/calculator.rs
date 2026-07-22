use crate::models::{Sale, PaidStatus};
use chrono::NaiveDate;

/// Commission percentage the agency earns from each airline, keyed by the
/// 2-character code used for the longest route on a ticket.
/// (code, display name, percent of system fare)
pub const AIRLINE_COMMISSIONS: &[(&str, &str, f64)] = &[
    ("ET", "Ethiopian", 6.0),
    ("WB", "Rwandair", 5.0),
    ("4Z", "Airlink", 3.0),
    ("TC", "Air Tanzania", 3.0),
    ("P0", "Proflight", 2.0),
    ("KQ", "Kenya Airways", 0.0),
    ("QR", "Qatar Airways", 7.0),
    ("EK", "Emirates", 6.0),
    ("SA", "South African", 9.0),
    ("BP", "Botswana", 4.0),
    ("TK", "Turkish Airlines", 7.0),
    ("ZN", "Zambia Airways", 5.0),
    ("UR", "Uganda Airlines", 3.0),
];

/// Looks up the commission percent for a route code. Unknown or blank
/// codes default to 0% rather than blocking the save — staff can fix the
/// code later and the amount recalculates automatically.
pub fn airline_commission_percent(code: &str) -> f64 {
    AIRLINE_COMMISSIONS
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, _, pct)| *pct)
        .unwrap_or(0.0)
}

/// Builds a Sale from the fields staff actually type in,
/// deriving fare_after_ob, balance, and airline_commission rather than
/// accepting them as input.
pub fn new_sale(
    date: NaiveDate,
    name: String,
    airline: String,
    ticket_number: Option<String>,
    phone: Option<String>,
    system_fare: f64,
    commission: f64,
    commission_route: Option<String>,
    amount_paid: f64,
    issued_by: String,
) -> Sale {
    let fare_after_ob = system_fare + commission;
    let balance = fare_after_ob - amount_paid;
    let airline_commission = system_fare
        * airline_commission_percent(commission_route.as_deref().unwrap_or(""))
        / 100.0;

    Sale {
        id: None,
        date, name, airline, ticket_number, phone,
        system_fare, commission, fare_after_ob,
        commission_route, airline_commission,
        amount_paid, balance, issued_by,
        created_at: None,
    }
}

/// Recompute the derived fields — call this any time a manual field changes
/// (e.g. staff edits commission or the route code after the fact), so
/// fare_after_ob/balance/airline_commission never drift out of sync with
/// their inputs.
pub fn recalculate(sale: &mut Sale) {
    sale.fare_after_ob = sale.system_fare + sale.commission;
    sale.balance = sale.fare_after_ob - sale.amount_paid;
    sale.airline_commission = sale.system_fare
        * airline_commission_percent(sale.commission_route.as_deref().unwrap_or(""))
        / 100.0;
}

pub fn paid_status(sale: &Sale) -> PaidStatus {
    if sale.balance <= 0.0 {
        PaidStatus::PaidInFull
    } else if sale.amount_paid > 0.0 {
        PaidStatus::Partial
    } else {
        PaidStatus::Unpaid
    }
}
