use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub date: NaiveDate,
    pub name: String,
    pub airline: String,
    pub ticket_number: Option<String>,
    pub phone: Option<String>,
    pub system_fare: f64,
    pub commission: f64,
    pub fare_after_ob: f64,
    // 2-character code (e.g. "SA", "ET") for the airline covering the
    // longest route on the ticket when two airlines are used — this drives
    // airline_commission below. None/blank means no route entered yet,
    // treated as 0% commission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission_route: Option<String>,
    // Auto-calculated: system_fare * (commission_route's %). Stored, not
    // just derived at display time, so historical rows keep their original
    // commission even if the percentage table changes later.
    pub airline_commission: f64,
    pub amount_paid: f64,
    pub balance: f64,
    pub issued_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PaidStatus {
    PaidInFull,
    Partial,
    Unpaid,
}
