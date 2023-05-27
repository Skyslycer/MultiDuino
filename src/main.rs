use lazy_static::__Deref;
use log::{error, info, warn};
use rand::{Rng, SeedableRng};

use chrono::Local;
use config::Config;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::sync::Arc;
use structs::DuinoConfig;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use sha1::{Digest, Sha1};
use std::time::Duration;

mod structs;
mod tui_main;

static RIG_NAME: &str = "None";

lazy_static::lazy_static! {
    pub static ref LOGGER: structs::VecLogger = structs::VecLogger::default();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    log::set_boxed_logger(Box::new(LOGGER.deref())).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let tui_accounts: Arc<RwLock<HashMap<String, structs::AccountData>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let tui_accounts_list: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    let settings = Config::builder()
        .add_source(config::File::with_name("conf.toml"))
        .build()
        .unwrap()
        .try_deserialize::<structs::DuinoConfig>()
        .unwrap();
    let mut global = structs::AccountData {
        hashrate: 0,
        miners: 0,
        connected: 0,
        current_balance: 0.0,
        status: "Gobal".to_string(),
        staked: 0.0,
        estimated_balance: 0.0,
        warnings: 0,
    };
    {
        let mut accounts = tui_accounts.write().await;
        let mut account_list = tui_accounts_list.write().await;
        for (name, account) in settings.accounts.iter() {
            let cloned_name = name.clone();
            let cloned_account = account.clone();
            if !check_user(name, &account.key).await {
                warn!(
                    "WARNING: Account {} either doesn't exist or has invalid mining key: {}",
                    &name, &account.key
                );
                let new_data = structs::AccountData {
                    hashrate: 0,
                    miners: 0,
                    connected: 0,
                    current_balance: 0.0,
                    status: "Not found".to_string(),
                    staked: 0.0,
                    estimated_balance: 0.0,
                    warnings: 0,
                };
                accounts.insert(cloned_name.clone(), new_data.clone());
                account_list.push(cloned_name.clone());
            } else {
                let account_data = get_user(&cloned_name).await;
                if account_data.success {
                    info!(
                        "SUCCESS: Account {} verified with mining key: {} Starting {} miners...",
                        &name, &account.key, &account.miners
                    );
                    let pool = get_pool().await;
                    for n in 1..account.miners + 1 {
                        let handle = tokio::spawn(mine(
                            pool.clone(),
                            format!("{}/{:03}", &name, n),
                            cloned_name.clone(),
                            cloned_account.clone(),
                        ));
                        handles.push(handle);
                    }
                    let new_data = structs::AccountData {
                        hashrate: account.miners as u16 * account.hashrate,
                        miners: account.miners,
                        connected: account_data.result.miners.len() as u8,
                        current_balance: account_data.result.balance.balance,
                        status: "Connected".to_string(),
                        staked: account_data.result.balance.stake_amount,
                        estimated_balance: account_data.result.balance.balance
                            * get_highest_amount(account_data.result.prices).await,
                        warnings: account_data.result.balance.warnings,
                    };
                    global.hashrate += new_data.hashrate;
                    global.miners += new_data.miners;
                    global.connected += new_data.connected;
                    global.current_balance += new_data.current_balance;
                    global.estimated_balance += new_data.estimated_balance;
                    global.warnings += new_data.warnings;
                    accounts.insert(cloned_name.clone(), new_data.clone());
                    account_list.push(cloned_name.clone());
                }
            }
        }
        accounts.insert("Global".to_string(), global.clone());
        account_list.push("Global".to_string());
    }
    tokio::spawn(run_update(
        Arc::clone(&tui_accounts),
        Arc::clone(&tui_accounts_list),
        settings.clone(),
    ));

    tui_main::init(tui_accounts, tui_accounts_list).await;
    for handle in handles {
        handle.await.expect("Await the task");
    }
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

async fn run_update(
    accounts: Arc<RwLock<HashMap<String, structs::AccountData>>>,
    account_list: Arc<RwLock<Vec<String>>>,
    settings: DuinoConfig,
) {
    loop {
        tokio::time::sleep(Duration::from_secs(settings.update_interval as u64)).await;
        let mut new_accounts: HashMap<String, structs::AccountData> = HashMap::new();
        let mut new_accounts_list: Vec<String> = Vec::new();

        let mut global = structs::AccountData {
            hashrate: 0,
            miners: 0,
            connected: 0,
            current_balance: 0.0,
            status: "Gobal".to_string(),
            staked: 0.0,
            estimated_balance: 0.0,
            warnings: 0,
        };
        for (name, account) in settings.accounts.iter() {
            let cloned_name = name.clone();
            if !check_user(name, &account.key).await {
                let new_data = structs::AccountData {
                    hashrate: 0,
                    miners: 0,
                    connected: 0,
                    current_balance: 0.0,
                    status: "Not found".to_string(),
                    staked: 0.0,
                    estimated_balance: 0.0,
                    warnings: 0,
                };
                new_accounts.insert(cloned_name.clone(), new_data.clone());
                new_accounts_list.push(cloned_name.clone());
            } else {
                let account_data = get_user(&cloned_name).await;
                if account_data.success {
                    let new_data = structs::AccountData {
                        hashrate: (account.miners as u16 * account.hashrate),
                        miners: account.miners,
                        connected: account_data.result.miners.len() as u8,
                        current_balance: account_data.result.balance.balance,
                        status: "Connected".to_string(),
                        staked: account_data.result.balance.stake_amount,
                        estimated_balance: account_data.result.balance.balance
                            * get_highest_amount(account_data.result.prices).await,
                        warnings: account_data.result.balance.warnings,
                    };
                    global.hashrate += new_data.hashrate;
                    global.miners += new_data.miners;
                    global.connected += new_data.connected;
                    global.current_balance += new_data.current_balance;
                    global.estimated_balance += new_data.estimated_balance;
                    global.warnings += new_data.warnings;
                    new_accounts.insert(cloned_name.clone(), new_data.clone());
                    new_accounts_list.push(cloned_name.clone());
                }
            }
        }
        new_accounts.insert("Global".to_string(), global.clone());
        new_accounts_list.push("Global".to_string());
        {
            let mut unlocked_accounts = accounts.write().await;
            let mut unlocked_account_list = account_list.write().await;
            unlocked_accounts.clear();
            unlocked_account_list.clear();
            unlocked_accounts.extend(new_accounts);
            unlocked_account_list.extend(new_accounts_list);
        }
    }
}

async fn get_highest_amount(amounts: HashMap<String, f64>) -> f64 {
    let mut current_highest = 0.0;
    for (_, price) in amounts {
        if price > current_highest {
            current_highest = price;
        }
    }
    current_highest
}

async fn check_user(name: &String, key: &String) -> bool {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0")
        .build()
        .expect("Built client");
    let mut retries = 0;
    loop {
        match client
            .get(format!(
                "https://server.duinocoin.com/mining_key?u={}&k={}",
                name, key
            ))
            .send()
            .await
        {
            Ok(response) => match response.json::<structs::AccountCheck>().await {
                Ok(account) => {
                    return account.success;
                }
                Err(_) => {
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    error!("Error decoding account check response, retrying... Attempt #{} Is the account banned?", retries);
                    continue;
                }
            },
            Err(_) => {
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                error!(
                    "Error making account check request, retrying... Attempt #{}",
                    retries
                );
                continue;
            }
        }
    }
}

async fn get_user(name: &String) -> structs::RestAccount {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0")
        .build()
        .expect("Built client");
    let mut retries = 0;
    loop {
        match client
            .get(format!("https://server.duinocoin.com/v3/users/{}", name))
            .send()
            .await
        {
            Ok(response) => match response.json::<structs::RestAccount>().await {
                Ok(account) => {
                    return account;
                }
                Err(_) => {
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    error!("Error decoding account get response, retrying... Attempt #{} Is the account banned?", retries);
                    continue;
                }
            },
            Err(_) => {
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                error!(
                    "Error making account get request, retrying... Attempt #{}",
                    retries
                );
                continue;
            }
        }
    }
}

async fn get_pool() -> structs::PoolResult {
    tokio::time::sleep(Duration::from_millis(250)).await;
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0")
        .build()
        .expect("Built client");
    let mut retries = 0;
    loop {
        match client
            .get("https://server.duinocoin.com/getPool")
            .send()
            .await
        {
            Ok(response) => match response.json::<structs::PoolResult>().await {
                Ok(pool) => {
                    return pool;
                }
                Err(_) => {
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    error!(
                        "Error decoding pool response, retrying... Attempt #{}",
                        retries
                    );
                    continue;
                }
            },
            Err(_) => {
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                error!(
                    "Error making pool request, retrying... Attempt #{}",
                    retries
                );
                continue;
            }
        }
    }
}

async fn mine(
    address: structs::PoolResult,
    miner_id: String,
    name: String,
    config: structs::Account,
) {
    let mut sock = loop {
        match tokio::net::TcpStream::connect(format!("{}:{}", address.ip, address.port)).await {
            Ok(stream) => {
                info!(
                    "{}: Connected to {}/{}!",
                    miner_id, address.server, address.name
                );
                break stream;
            }
            Err(_) => {
                warn!("{}: Unable to setup mining node! Retrying in 5s", miner_id);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };
    let mut bufread = BufReader::new(&mut sock);

    let mut rng = StdRng::from_entropy();
    let ducoid = format!("DUCOID{:08X}{:08X}", rng.gen::<u32>(), rng.gen::<u32>());

    let mut version_buffer = Vec::new();
    bufread
        .read_until(0x0A, &mut version_buffer)
        .await
        .expect("Couldn't read version buffer.");

    let mut num_shares = 0;
    let mut num_good_shares = 0;

    loop {
        let job = format!("JOB,{},AVR,{}", name, config.key);
        bufread.write_all(job.as_bytes()).await.expect("Write job");

        let mut job_data_buffer = Vec::new();
        bufread
            .read_until(0x0A, &mut job_data_buffer)
            .await
            .expect("Couldn't read version buffer.");

        let untrimmed_job_data = String::from_utf8_lossy(&job_data_buffer).to_string();
        let job_data = untrimmed_job_data
            .trim_matches(char::from(0))
            .trim()
            .split(',')
            .collect::<Vec<&str>>();
        if job_data.len() != 3 {
            warn!("ERROR: Invalid job data: {}", untrimmed_job_data);
            continue;
        }
        let difficulty = job_data.get(2).unwrap().trim().parse::<u16>().unwrap();

        let res = ducos1a(
            job_data.first().expect("Expected lastblockhash"),
            job_data.get(1).expect("Expected newblockhash"),
            difficulty,
            1000 / config.hashrate as u64,
        )
        .await;
        num_shares += 1;

        let result = format!(
            "{},{},Official AVR Miner 3.5,{},{}",
            res, config.hashrate, &RIG_NAME, ducoid
        );
        bufread
            .write_all(result.as_bytes())
            .await
            .expect("Write the result");

        let mut feedback = Vec::new();
        bufread
            .read_until(0x0A, &mut feedback)
            .await
            .expect("Read feedback successfully");
        let untrimmed_feedback_str = String::from_utf8_lossy(&feedback);
        let feedback_str = untrimmed_feedback_str.trim_matches(char::from(0)).trim();
        let feedback_sanitized = match feedback_str {
            "GOOD" | "BLOCK" => {
                num_good_shares += 1;
                "Accepted"
            }
            _ => "Rejected",
        };
        info!(
            "[{}] {}: [{}] {}/{} shares | {} H/s | {} difficulty",
            Local::now().format("%H:%M:%S"),
            miner_id,
            feedback_sanitized,
            num_good_shares,
            num_shares,
            config.hashrate,
            difficulty
        );
    }
}

async fn ducos1a(lastblockhash: &str, newblockhash: &str, difficulty: u16, hash_time: u64) -> u16 {
    let mut job = vec![0; 20];
    for (i, j) in (0..40).step_by(2).zip(0..20) {
        let a = newblockhash.as_bytes()[i] & 0x1F;
        let b = newblockhash.as_bytes()[i + 1] & 0x1F;
        job[j] = (((a + 9) % 25) << 4) + ((b + 9) % 25);
    }
    for ducos1res in 0..=difficulty * 100 + 1 {
        let mut hasher = Sha1::new();
        let data = format!("{}{}", lastblockhash, ducos1res);
        hasher.update(data.as_bytes());
        let hash_bytes = hasher.finalize();

        if hash_bytes.as_slice() == job {
            return ducos1res;
        }
        tokio::time::sleep(Duration::from_micros(hash_time)).await;
    }
    0
}
