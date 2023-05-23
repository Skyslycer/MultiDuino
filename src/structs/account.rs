use std::collections::HashMap;

use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

// REST Account
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestAccount {
    pub result: AccountResult,
    pub server: String,
    pub success: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountResult {
    pub achievements: Vec<i64>,
    pub balance: AccountBalance,
    pub items: Vec<Value>,
    pub miners: Vec<Value>,
    pub prices: HashMap<String, f64>,
    pub transactions: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountBalance {
    pub balance: f64,
    pub created: String,
    pub last_login: u64,
    pub stake_amount: f64,
    pub stake_date: u64,
    pub trust_score: u64,
    pub username: String,
    pub verified: String,
    pub verified_by: String,
    pub verified_date: u64,
    pub warnings: u32,
}

// TUI Account
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct AccountData {
    pub hashrate: u16,
    pub miners: u8,
    pub connected: u8,
    pub current_balance: f64,
    pub status: String,
    pub estimated_balance: f64,
    pub staked: f64,
    pub warnings: u32,
}

// Config Account
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub key: String,
    pub hashrate: u16,
    pub miners: u8
}

// Account check
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct AccountCheck {
    pub has_key: bool,
    pub success: bool
}