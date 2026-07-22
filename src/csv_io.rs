use anyhow::Result;
use crate::models::Sale;
use std::fs;
use std::path::Path;

// Excel auto-detects "numeric-looking" CSV cells and reformats them as
// numbers on open — which mangles anything with a leading zero (phone
// numbers, some ticket numbers). Wrapping the value as ="..." makes Excel
// treat it as a text-returning formula instead, so it always displays
// exactly as typed. Only applied to non-empty values; an empty field stays
// empty rather than becoming a pointless ="" cell.
fn excel_text(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("=\"{}\"", value)
    }
}

pub fn export_csv(path: &str, sales: &[Sale]) -> Result<()> {

    //check if le path exists
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(&[
        "DATE", "NAME", "AIRLINE", "TICKET NUMBER", "PHONE",
        "SYSTEM FARE ZMW", "COMMISSION", "FARE AFTER OB ZMW",
        "AMOUNT PAID", "BALANCE", "ISSUED BY",
    ])?;
    for s in sales {
        wtr.write_record(&[
            s.date.to_string(),
            s.name.clone(),
            s.airline.clone(),
            excel_text(&s.ticket_number.clone().unwrap_or_default()),
            excel_text(&s.phone.clone().unwrap_or_default()),
            s.system_fare.to_string(),
            s.commission.to_string(),
            s.fare_after_ob.to_string(),
            s.amount_paid.to_string(),
            s.balance.to_string(),
            s.issued_by.clone(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

// NOTE: real rows have gaps (missing commission, missing fare_after_ob on
// unpriced bookings) — this parses leniently and recalculates derived
// fields rather than trusting whatever was typed in those columns.
// Historical May/June data has no PHONE column, so this always imports
// phone as None — fine, since that data predates the field entirely.
pub fn import_csv(path: &str) -> Result<Vec<Sale>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut sales = vec![];
    for result in rdr.records() {
        let record = result?;
        let system_fare: f64 = record.get(4).unwrap_or("0").parse().unwrap_or(0.0);
        let commission: f64 = record.get(5).unwrap_or("0").parse().unwrap_or(0.0);
        let amount_paid: f64 = record.get(7).unwrap_or("0").parse().unwrap_or(0.0);

        let mut sale = crate::calculator::new_sale(
            chrono::NaiveDate::parse_from_str(record.get(0).unwrap_or(""), "%Y-%m-%d")
                .unwrap_or_default(),
            record.get(1).unwrap_or("").to_string(),
            record.get(2).unwrap_or("").to_string(),
            record.get(3).filter(|s| !s.is_empty()).map(String::from),
            None, // phone — not present in historical data
            system_fare,
            commission,
            amount_paid,
            record.get(9).unwrap_or("").to_string(),
        );
        crate::calculator::recalculate(&mut sale);
        sales.push(sale);
    }
    Ok(sales)
}
