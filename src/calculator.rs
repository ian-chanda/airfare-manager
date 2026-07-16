use crate::models::{Sale, PaidStatus};
use chrono::NaiveDate;

/// Builds a Sale from the fields staff actually type in,
/// deriving fare_after_ob and balance rather than accepting them as input.
pub fn new_sale(
    date: NaiveDate,
    name: String,
    airline: String,
    ticket_number: Option<String>,
    phone: Option<String>,
    system_fare: f64,
    commission: f64,
    amount_paid: f64,
    issued_by: String,
) -> Sale {
    let fare_after_ob = system_fare + commission;
    let balance = fare_after_ob - amount_paid;

    Sale {
        id: None,
        date, name, airline, ticket_number, phone,
        system_fare, commission, fare_after_ob,
        amount_paid, balance, issued_by,
        created_at: None,
    }
}

/// Recompute the derived fields — call this any time a manual field changes
/// (e.g. staff edits commission after the fact), so fare_after_ob/balance
/// never drift out of sync with their inputs.
pub fn recalculate(sale: &mut Sale) {
    sale.fare_after_ob = sale.system_fare + sale.commission;
    sale.balance = sale.fare_after_ob - sale.amount_paid;
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
