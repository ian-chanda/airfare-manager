use anyhow::Result;
use reqwest::blocking::Client;
use crate::models::Sale;

pub struct Db {
    client: Client,
    url: String,
    key: String,
}

impl Db {
    pub fn new(url: String, key: String) -> Self {
        Self { client: Client::new(), url, key }
    }

    pub fn save(&self, sale: &Sale) -> Result<()> {
        self.client
            .post(format!("{}/rest/v1/sales", self.url))
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .json(sale)
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn fetch_all(&self) -> Result<Vec<Sale>> {
        let res = self.client
            .get(format!("{}/rest/v1/sales?select=*", self.url))
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .send()?
            .error_for_status()?
            .json()?;
        Ok(res)
    }

    /// Updates a single existing row, targeted by its id.
    /// Errors out early if the sale has no id (i.e. was never saved yet),
    /// since there'd be nothing to target in Supabase.
    pub fn update_paid(&self, id: i64, amount_paid: f64, balance: f64) -> Result<()> {
        let body = serde_json::json!({
            "amount_paid": amount_paid,
            "balance": balance,
        });

        let res = self.client
            .patch(format!("{}/rest/v1/sales?id=eq.{}", self.url, id))
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .json(&body)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().unwrap_or_default();
            return Err(anyhow::anyhow!("Supabase update failed ({}): {}", status, text));
        }

        Ok(())
    }
}
