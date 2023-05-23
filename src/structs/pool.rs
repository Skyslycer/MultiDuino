use serde_derive::Serialize;
use serde_derive::Deserialize;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct PoolResult {
    pub client: String,
    pub ip: String,
    pub name: String,
    pub port: u64,
    pub region: String,
    pub server: String,
    pub success: bool
}