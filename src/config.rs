pub(crate) struct AppConfig {
    pub(crate) address: String,
    pub(crate) debug: bool,
}

impl AppConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            address: std::env::var("APP_ADDR").unwrap_or_else(|_| "0.0.0.0:13000".to_owned()),
            debug: std::env::var("DEBUG")
                .is_ok_and(|value| value.trim().eq_ignore_ascii_case("true")),
        }
    }
}
