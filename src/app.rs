use crate::config::Config;
use crate::csv_io;
use crate::db::Db;
use crate::models::Sale;
use crate::calculator;
use chrono::{Datelike, NaiveDate};
use egui_extras::{TableBuilder, Column};
use num_format::{Locale, ToFormattedString};
use std::collections::HashMap;

#[derive(PartialEq)]
enum Tab {
    NewSale,
    AllSales,
    Summary,
}

pub struct TaxApp {
    db: Db,
    tab: Tab,

    // Access gate — one shared password for the whole app (temporary, v1 only)
    unlocked: bool,
    password_input: String,
    login_error: String,
    app_password: String,

    // New Sale form fields
    date: String,
    name: String,
    airline: String,
    ticket_number: String,
    phone: String,
    system_fare: String,
    commission: String,
    // Airline commission percentage, typed in directly by staff (e.g. "6" for 6%).
    commission_percent: String,
    amount_paid: String,
    issued_by: String,
    status: String,

    // All Sales / Summary cache — fetched on demand, not every frame
    sales: Vec<Sale>,
    sales_status: String,
    search_query: String,

    // Row-level edit buffers for the "Paid" column, keyed by sale id.
    // Needed because the column shows a formatted f64, but while the
    // user is typing we want their raw text, not a reformatted value.
    paid_edit_buffer: HashMap<i64, String>,
}

// Accent color used for the primary action button and selected nav item.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(116, 161, 46); // green


// Formats a money amount with thousands separators and 2 decimal places,
// e.g. 10000.0 -> "10,000.00". Prefix with "K" at the call site as needed
// (kept separate so callers can also use this for plain numeric displays).
fn fmt_money(amount: f64) -> String {
    let sign = if amount < 0.0 { "-" } else { "" };
    let abs = amount.abs();
    // Round to whole cents first so the grouping/formatting below is exact
    // and never off-by-one-cent from float rounding at display time.
    let cents = (abs * 100.0).round() as i64;
    let whole = cents / 100;
    let frac = cents % 100;
    format!(
        "{}{}.{:02}",
        sign,
        whole.to_formatted_string(&Locale::en),
        frac
    )
}

// All look-and-feel customization lives here, in one place, so it can be
// called both once at startup AND every frame. This matters because egui
// swaps in a brand-new Visuals object whenever the OS theme changes
// (light <-> dark), which would otherwise wipe out these customizations
// the moment someone's system theme flips.
fn apply_custom_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(10.0, 1.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    let radius = egui::CornerRadius::same(5);
    style.visuals.widgets.noninteractive.corner_radius = radius;
    style.visuals.widgets.inactive.corner_radius = radius;
    style.visuals.widgets.hovered.corner_radius = radius;

    if style.visuals.dark_mode {
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(40, 40, 40);
    }
    else{
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(230, 230, 230);
    }

    style.visuals.widgets.active.corner_radius = radius;

    // Tint the selection color (used for selected nav item, text selection, etc.)
    // to match the accent instead of egui's default blue-gray.
    style.visuals.selection.bg_fill = ACCENT;
}

impl TaxApp {
    pub fn new(config: Config, cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_zoom_factor(1.10);

        let mut style = (*cc.egui_ctx.global_style()).clone();
        apply_custom_style(&mut style);
        cc.egui_ctx.set_global_style(style);

        let app_password = config.app_password.clone();
        let db = Db::new(config.supabase_url, config.supabase_key);

        Self {
            db,
            tab: Tab::NewSale,
            unlocked: false,
            password_input: String::new(),
            login_error: String::new(),
            app_password,
            date: String::new(),
            name: String::new(),
            airline: String::new(),
            ticket_number: String::new(),
            phone: String::new(),
            system_fare: String::new(),
            commission: String::new(),
            commission_percent: String::new(),
            amount_paid: String::new(),
            issued_by: String::new(),
            status: String::new(),
            sales: Vec::new(),
            sales_status: String::new(),
            search_query: String::new(),
            paid_edit_buffer: HashMap::new(),
        }
    }

    fn login_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.add(
                egui::Image::new(egui::include_image!("../assets/Logo12.png"))
                    .max_height(300.0)
                );
            ui.add_space(5.0);

            egui::Frame::group(ui.style())
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.set_max_width(260.0);
                    ui.label("Enter password to continue:");
                    ui.add_space(6.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.password_input).password(true),
                    );
                    let enter_pressed =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    ui.add_space(8.0);
                    let clicked = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Unlock").color(egui::Color32::WHITE),
                            )
                            .fill(ACCENT),
                        )
                        .clicked();

                    if clicked || enter_pressed {
                        if self.password_input == self.app_password {
                            self.unlocked = true;
                            self.login_error.clear();
                        } else {
                            self.login_error = "Wrong password".into();
                        }
                        self.password_input.clear();
                    }

                    if !self.login_error.is_empty() {
                        ui.add_space(6.0);
                        ui.colored_label(egui::Color32::from_rgb(200, 40, 40), &self.login_error);
                    }
                });
        });
    }

    fn save_sale(&mut self) {
        let date = match NaiveDate::parse_from_str(&self.date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                self.status = "Invalid date — use YYYY-MM-DD".into();
                return;
            }
        };
        let system_fare: f64 = match self.system_fare.parse() {
            Ok(v) => v,
            Err(_) => {
                self.status = "System Fare must be a number".into();
                return;
            }
        };
        let commission: f64 = match self.commission.parse() {
            Ok(v) => v,
            Err(_) => {
                self.status = "Commission must be a number".into();
                return;
            }
        };
        let airline_commission_percent: f64 = if self.commission_percent.trim().is_empty() {
            0.0
        } else {
            match self.commission_percent.parse() {
                Ok(v) => v,
                Err(_) => {
                    self.status = "Airline Commission % must be a number".into();
                    return;
                }
            }
        };
        let amount_paid: f64 = match self.amount_paid.parse() {
            Ok(v) => v,
            Err(_) => {
                self.status = "Amount Paid must be a number".into();
                return;
            }
        };
        let ticket_number = if self.ticket_number.trim().is_empty() {
            None
        } else {
            Some(self.ticket_number.clone())
        };
        let phone = if self.phone.trim().is_empty() {
            None
        } else {
            Some(self.phone.clone())
        };

        let sale = calculator::new_sale(
            date,
            self.name.clone(),
            self.airline.clone(),
            ticket_number,
            phone,
            system_fare,
            commission,
            airline_commission_percent,
            amount_paid,
            self.issued_by.clone(),
        );

        match self.db.save(&sale) {
            Ok(_) => {
                self.status = "Saved!".into();
                self.date.clear();
                self.name.clear();
                self.airline.clear();
                self.ticket_number.clear();
                self.phone.clear();
                self.system_fare.clear();
                self.commission.clear();
                self.commission_percent.clear();
                self.amount_paid.clear();
                self.issued_by.clear();
                self.refresh_sales(); // keep All Sales / Summary in sync after a save
            }
            Err(e) => self.status = format!("Save failed: {}", e),
        }
    }

    fn refresh_sales(&mut self) {
        match self.db.fetch_all() {
            Ok(sales) => {
                self.sales_status = format!("{} sales loaded", sales.len());
                self.sales = sales;
                self.paid_edit_buffer.clear();
            }
            Err(e) => self.sales_status = format!("Fetch failed: {}", e),
        }
    }

    fn export_sales(&mut self) {
        let filename = format!(
            "exports/sales_export_{}.csv",
            chrono::Local::now().format("%Y-%m-%d_%H%M%S")
        );
        match csv_io::export_csv(&filename, &self.sales) {
            Ok(_) => self.sales_status = format!("Exported to {}", filename),
            Err(e) => self.sales_status = format!("Export failed: {}", e),
        }
    }

    fn new_sale_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("New Sale");
        ui.add_space(12.0);

        egui::Frame::group(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                egui::Grid::new("new_sale_grid")
                    .num_columns(2)
                    .spacing([12.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Date (YYYY-MM-DD):");
                        ui.text_edit_singleline(&mut self.date);
                        ui.end_row();

                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.name);
                        ui.end_row();

                        ui.label("Phone:");
                        ui.text_edit_singleline(&mut self.phone);
                        ui.end_row();

                        ui.label("Airline:");
                        ui.text_edit_singleline(&mut self.airline);
                        ui.end_row();

                        ui.label("Ticket Number:");
                        ui.text_edit_singleline(&mut self.ticket_number);
                        ui.end_row();

                        ui.label("System Fare:");
                        ui.text_edit_singleline(&mut self.system_fare);
                        ui.end_row();

                        ui.label("Commission:");
                        ui.text_edit_singleline(&mut self.commission);
                        ui.end_row();

                        ui.label("Airline Commission %:");
                        ui.text_edit_singleline(&mut self.commission_percent);
                        ui.end_row();

                        ui.label("Amount Paid:");
                        ui.text_edit_singleline(&mut self.amount_paid);
                        ui.end_row();

                        ui.label("Issued By:");
                        ui.text_edit_singleline(&mut self.issued_by);
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                if let (Ok(sf), Ok(c)) = (
                    self.system_fare.parse::<f64>(),
                    self.commission.parse::<f64>(),
                ) {
                    let fare_after_ob = sf + c;
                    ui.label(format!("Fare After OB (auto): K{}", fmt_money(fare_after_ob)));

                    // Blank is treated as 0% for the preview; an invalid
                    // (non-numeric) entry just skips the preview line rather
                    // than guessing — save_sale() will catch it properly.
                    let pct = if self.commission_percent.trim().is_empty() {
                        Some(0.0)
                    } else {
                        self.commission_percent.parse::<f64>().ok()
                    };
                    if let Some(pct) = pct {
                        let airline_commission = sf * pct / 100.0;
                        ui.label(format!(
                            "Airline Commission (auto): K{}",
                            fmt_money(airline_commission)
                        ));
                    }

                    if let Ok(paid) = self.amount_paid.parse::<f64>() {
                        ui.label(format!(
                            "Balance (auto): K{}",
                            fmt_money(fare_after_ob - paid)
                        ));
                    }
                }

                ui.add_space(8.0);

                let save_button = egui::Button::new(
                    egui::RichText::new("Save Sale").color(egui::Color32::WHITE),
                )
                .fill(ACCENT);

                if ui.add(save_button).clicked() {
                    self.save_sale();
                }
                if !self.status.is_empty() {
                    ui.add_space(6.0);
                    ui.label(&self.status);
                }
            });
    }

    fn all_sales_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("All Sales");
        ui.add_space(12.0);

        egui::Frame::group(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        self.refresh_sales();
                    }
                    if ui.button("Export CSV").clicked() {
                        self.export_sales();
                    }
                    if !self.sales_status.is_empty() {
                        ui.label(&self.sales_status);
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                });
                ui.add_space(8.0);

                let query = self.search_query.to_lowercase();
                let filtered: Vec<&Sale> = self
                    .sales
                    .iter()
                    .filter(|s| {
                        if query.is_empty() {
                            return true;
                        }
                        s.name.to_lowercase().contains(&query)
                            || s.airline.to_lowercase().contains(&query)
                            || s.phone
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(&query)
                    })
                    .collect();

                let row_height = 26.0;
                // Set by the Paid-column edit box when the user commits a change.
                // Applied to self.sales AFTER the table (and its borrow of
                // self.sales via `filtered`) is done being built.
                let mut pending_update: Option<(i64, f64)> = None;

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .column(Column::initial(100.0).resizable(true)) // date
                    .column(Column::remainder().at_least(120.0)) // name — absorbs leftover space
                    .column(Column::initial(100.0).resizable(true)) // phone
                    .column(Column::initial(90.0).resizable(true)) // airline
                    .column(Column::initial(100.0).resizable(true)) // system_fare
                    .column(Column::initial(100.0).resizable(true)) // commission
                    .column(Column::initial(110.0).resizable(true)) // fare_after_ob
                    .column(Column::initial(70.0).resizable(true)) // airline_commission_percent
                    .column(Column::initial(110.0).resizable(true)) // airline_commission
                    .column(Column::initial(90.0).resizable(true)) // amount_paid (editable)
                    .column(Column::initial(90.0).resizable(true)) // balance
                    .column(Column::initial(90.0).resizable(true)) // issued_by
                    .header(22.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Date");
                        });
                        header.col(|ui| {
                            ui.strong("Name");
                        });
                        header.col(|ui| {
                            ui.strong("Phone");
                        });
                        header.col(|ui| {
                            ui.strong("Airline");
                        });
                        header.col(|ui| {
                            ui.strong("System Fare");
                        });
                        header.col(|ui| {
                            ui.strong("Commission");
                        });
                        header.col(|ui| {
                            ui.strong("Fare After OB");
                        });
                        header.col(|ui| {
                            ui.strong("Comm. %");
                        });
                        header.col(|ui| {
                            ui.strong("Airline Comm.");
                        });
                        header.col(|ui| {
                            ui.strong("Paid");
                        });
                        header.col(|ui| {
                            ui.strong("Balance");
                        });
                        header.col(|ui| {
                            ui.strong("Issued By");
                        });
                    })
                    .body(|body| {
                        body.rows(row_height, filtered.len(), |mut row| {
                            let sale = filtered[row.index()];
                            row.col(|ui| {
                                ui.label(sale.date.to_string());
                            });
                            row.col(|ui| {
                                ui.label(&sale.name);
                            });
                            row.col(|ui| {
                                ui.label(sale.phone.as_deref().unwrap_or(""));
                            });
                            row.col(|ui| {
                                ui.label(&sale.airline);
                            });
                            row.col(|ui| {
                                ui.label(format!("K{}", fmt_money(sale.system_fare)));
                            });
                            row.col(|ui| {
                                ui.label(format!("K{}", fmt_money(sale.commission)));
                            });
                            row.col(|ui| {
                                ui.label(format!("K{}", fmt_money(sale.fare_after_ob)));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2}%", sale.airline_commission_percent));
                            });
                            row.col(|ui| {
                                ui.label(format!("K{}", fmt_money(sale.airline_commission)));
                            });
                            row.col(|ui| {
                                if let Some(id) = sale.id {
                                    // Kept as a raw plain number (no commas) while editing —
                                    // easier to type into and to re-parse as f64 on commit.
                                    let buffer = self
                                        .paid_edit_buffer
                                        .entry(id)
                                        .or_insert_with(|| format!("{:.2}", sale.amount_paid));

                                    let response = ui.add(
                                        egui::TextEdit::singleline(buffer).desired_width(65.0),
                                    );

                                    let committed = response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                    if committed {
                                        match buffer.trim().parse::<f64>() {
                                            Ok(new_paid) => {
                                                pending_update = Some((id, new_paid));
                                            }
                                            Err(_) => {
                                                // Bad input — reset to the last known-good value.
                                                *buffer = format!("{:.2}", sale.amount_paid);
                                            }
                                        }
                                    }
                                } else {
                                    // No id means this row was never saved — shouldn't
                                    // normally appear here, but don't offer editing on it.
                                    ui.label(format!("K{}", fmt_money(sale.amount_paid)));
                                }
                            });
                            row.col(|ui| {
                                ui.label(format!("K{}", fmt_money(sale.balance)));
                            });
                            row.col(|ui| {
                                ui.label(&sale.issued_by);
                            });
                        });
                    });

                // filtered's borrow of self.sales has ended by this point, so
                // it's safe to mutate self.sales / call self.db here.
                if let Some((id, new_paid)) = pending_update {
                    if let Some(idx) = self.sales.iter().position(|s| s.id == Some(id)) {
                        let new_balance = self.sales[idx].fare_after_ob - new_paid;

                        match self.db.update_paid(id, new_paid, new_balance) {
                            Ok(_) => {
                                self.sales[idx].amount_paid = new_paid;
                                self.sales[idx].balance = new_balance;
                                self.paid_edit_buffer.remove(&id);
                                self.sales_status = "Paid amount updated".into();
                            }
                            Err(e) => {
                                self.sales_status = format!("Update failed: {}", e);
                            }
                        }
                    }
                }
            });
    }

    fn summary_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Summary");
        ui.add_space(12.0);

        egui::Frame::group(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                if self.sales.is_empty() {
                    ui.label("No data loaded — visit All Sales and hit Refresh first.");
                    return;
                }

                let total_count = self.sales.len();
                let total_revenue: f64 = self.sales.iter().map(|s| s.fare_after_ob).sum();
                let total_outstanding: f64 = self
                    .sales
                    .iter()
                    .map(|s| s.balance)
                    .filter(|b| *b > 0.0)
                    .sum();
                let total_airline_commission: f64 =
                    self.sales.iter().map(|s| s.airline_commission).sum();

                ui.label(format!("Total sales: {}", total_count));
                ui.label(format!("Total revenue: K{}", fmt_money(total_revenue)));
                ui.label(format!(
                    "Total outstanding balance: K{}",
                    fmt_money(total_outstanding)
                ));
                ui.label(format!(
                    "Total airline commission: K{}",
                    fmt_money(total_airline_commission)
                ));

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading("By issuing agency");
                ui.add_space(6.0);

                let mut by_staff: HashMap<String, (u32, f64)> = HashMap::new();
                for sale in &self.sales {
                    let entry = by_staff.entry(sale.issued_by.clone()).or_insert((0, 0.0));
                    entry.0 += 1;
                    entry.1 += sale.fare_after_ob;
                }

                // HashMap iteration order isn't stable — sort for consistent display
                let mut staff_rows: Vec<(&String, &(u32, f64))> = by_staff.iter().collect();
                staff_rows.sort_by(|a, b| a.0.cmp(b.0));

                for (staff, (count, revenue)) in staff_rows {
                    ui.label(format!(
                        "{}: {} sales, K{} revenue",
                        staff,
                        count,
                        fmt_money(*revenue)
                    ));
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading("Monthly sales");
                ui.add_space(6.0);

                // Group by (year, month) pulled straight off each sale's date.
                // Same pattern as the by-staff grouping above, just a different key.
                let mut by_month: HashMap<(i32, u32), (u32, f64)> = HashMap::new();
                for sale in &self.sales {
                    let key = (sale.date.year(), sale.date.month());
                    let entry = by_month.entry(key).or_insert((0, 0.0));
                    entry.0 += 1;
                    entry.1 += sale.fare_after_ob;
                }

                let mut month_rows: Vec<((i32, u32), (u32, f64))> =
                    by_month.into_iter().collect();
                // Most recent month first.
                month_rows.sort_by(|a, b| b.0.cmp(&a.0));

                const MONTH_NAMES: [&str; 12] = [
                    "January", "February", "March", "April", "May", "June",
                    "July", "August", "September", "October", "November", "December",
                ];

                for ((year, month), (count, revenue)) in month_rows {
                    let month_name = MONTH_NAMES[(month - 1) as usize];
                    ui.label(format!(
                        "{} {}: {} sales, K{} revenue",
                        month_name,
                        year,
                        count,
                        fmt_money(revenue)
                    ));
                }
            });
    }
}

impl eframe::App for TaxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Re-apply our custom styling every frame. This matters because egui
        // swaps in a fresh Visuals object whenever the OS theme flips between
        // light and dark — without this, the customizations set once in
        // TaxApp::new only survive until the first theme change.
        ui.ctx().style_mut(apply_custom_style);

        if !self.unlocked {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.login_ui(ui);
            });
            return;
        }

        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.add_space(5.0);
                ui.heading("Palm Travel");
                ui.add_space(5.0);

                ui.selectable_value(&mut self.tab, Tab::NewSale, "  New Sale");
                ui.add_space(4.0);
                ui.selectable_value(&mut self.tab, Tab::AllSales, "  All Sales");
                ui.add_space(4.0);
                ui.selectable_value(&mut self.tab, Tab::Summary, "  Summary");
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            match self.tab {
                Tab::NewSale => self.new_sale_ui(ui),
                Tab::AllSales => self.all_sales_ui(ui),
                Tab::Summary => self.summary_ui(ui),
            }
        });
    }
}
