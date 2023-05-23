use config::Map;
use serde_derive::Serialize;
use serde_derive::Deserialize;

use super::account;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct DuinoConfig {
    pub update_interval: u32,
    pub accounts: Map<String, account::Account>
}