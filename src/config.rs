pub struct Config {
    pub supabase_url: String,
    pub supabase_key: String,
    pub app_password: String,
}

impl Config {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        Self {
            supabase_url: std::env::var("SUPABASE_URL").expect("SUPABASE_URL not set"),
            supabase_key: std::env::var("SUPABASE_KEY").expect("SUPABASE_KEY not set"),
            app_password: std::env::var("APP_PASSWORD").expect("APP_PASSWORD not set"),
        }
    }
}
