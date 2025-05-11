use std::{collections::HashMap, sync::OnceLock};

use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub namespace_label_selector: Option<HashMap<String, String>>,
    pub same_namespace: bool,
}

pub fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| {
        ::config::Config::builder()
            .add_source(::config::File::with_name("controller"))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    })
}
