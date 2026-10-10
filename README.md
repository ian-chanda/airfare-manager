# Airfare Manager

Airfare Manager is a **Rust desktop application** developed for **Palm Travel** to
streamline ticket sales recording, commission calculations, payment tracking, and
financial reporting. Records are synchronized with a hosted Supabase (PostgreSQL)
database so they are never locked to a single machine.

The application is written in Rust with an `egui`/`eframe` native GUI and talks to the
backend over the Supabase REST (PostgREST) API.

## Features

- **Ticket sales recording** — enter date, customer, phone, airline, ticket number,
  system fare, commission, airline commission percentage, amount paid, and issuing staff.
- **Commission calculations** — automatically derives fare-after-OB, the airline
  commission amount, and the outstanding balance from the values staff type in.
- **Payment tracking** — the *Paid* column in the sales table is editable inline; committing
  a value updates the record and recalculates the balance.
- **Supabase synchronization** — records are saved to and fetched from a hosted Supabase
  database via the REST API.
- **Search** — filter loaded sales by customer name, airline, or phone.
- **CSV export** — export the current sales list to a spreadsheet-friendly CSV file.
- **Financial summaries** — totals (sales count, revenue, outstanding balance, airline
  commission), plus breakdowns by issuing agency and by month.

## Technical notes

Three implementation details worth calling out:

1. **Centralized derived-field recalculation.** `src/calculator.rs` computes
   `fare_after_ob`, `balance`, and `airline_commission` in one place (`new_sale` /
   `recalculate`) instead of scattering the arithmetic across the UI, so the displayed
   figures can never drift out of sync with their inputs. CSV import reuses the same
   recalculation path rather than trusting stored values.
2. **Borrow-checker-safe inline editing.** The editable *Paid* cell uses an edit buffer
   keyed by record id and defers the database write until after the table's immutable
   borrow of the sales list has ended (`src/app.rs`), avoiding an aliasing conflict while
   still updating the UI immediately.
3. **Excel-safe CSV export.** Non-empty values are written as `="..."` formula-style text
   so Excel preserves leading zeros in phone and ticket numbers instead of reformatting
   them as numbers (`src/csv_io.rs`).

## Technology stack

- **Language:** Rust (edition 2024; requires Rust 1.85+)
- **GUI:** `eframe` 0.34 / `egui` 0.34 (glow renderer), `egui_extras` for tables
- **HTTP client:** `reqwest` 0.12 (blocking + JSON)
- **Serialization:** `serde` / `serde_json`
- **CSV:** `csv` 1
- **Dates/time:** `chrono`
- **Errors:** `anyhow`
- **Number formatting:** `num-format`
- **Assets/icons:** `image`, `include_bytes!` / `include_image!`
- **Configuration:** `dotenvy`
- **Backend:** Supabase (PostgreSQL) over the PostgREST REST API

## Setup

1. Install Rust (1.85 or newer) from <https://rustup.rs>.
2. Clone the repository, then create your local environment file:

   ```sh
   cp .env.example .env
   ```

3. Fill in the three required variables in `.env` (see below).
4. Build and run:

   ```sh
   cargo run --release
   ```

### Environment variables

| Variable | Purpose |
| --- | --- |
| `SUPABASE_URL` | Base URL of the Supabase project REST endpoint |
| `SUPABASE_KEY` | Supabase API key used by the desktop client |
| `APP_PASSWORD` | Shared password required to unlock the app |

Placeholder values live in `.env.example`. Real credentials belong only in `.env`, which
is git-ignored and must never be committed.

**Building vs. running.** The project requires all three environment variables to be
defined in order to compile. Actually *using* the application also requires valid
credentials and a configured Supabase database (a `sales` table with the expected
columns) — the placeholder values are enough to build, but not to run against a backend.

**Test data.** The `sales_export_*.csv` files in the repository root and `src/` contain
synthetic test/export data only — they are not real customer records.

## Screenshots

<img width="1680" height="1050" alt="Screenshot 2026-10-10 at 6 49 49 pm" src="https://github.com/user-attachments/assets/7b6ca090-23db-4a65-ac3e-5f9af43a6397" />
<img width="1680" height="1050" alt="Screenshot 2026-10-10 at 6 51 18 pm" src="https://github.com/user-attachments/assets/d779900a-4e4d-4d33-8598-e27b2ba69c2f" />
<img width="1586" height="992" alt="Palm Travel Sales Dashboard" src="https://github.com/user-attachments/assets/ca874108-0dd8-4918-82fb-7bd1bdb094ae" />





## Limitations

- **Shared-password authentication.** Access control is a single shared password with no
  per-user accounts or roles.
- **CSV import is not exposed in the UI.** `csv_io::import_csv` is implemented, but there
  is no button or menu entry that invokes it — only export is reachable from the interface.
- **Limited record editing.** Records can be created and their *Paid* value edited, but
  there is no UI to edit or delete other fields once a record is saved.
- **No offline support.** The app requires network access and fetches the full sales list
  from Supabase on each refresh; there is no local cache or offline mode.
