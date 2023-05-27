use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use log::{Metadata, Record};

static MAX_LOGS: usize = 1000;

#[derive(Default)]
pub struct VecLogger {
    logs: Arc<Mutex<VecDeque<String>>>,
}

impl VecLogger {
    pub fn logs(&self) -> VecDeque<String> {
        self.logs.lock().unwrap().clone()
    }

    pub fn log(&self, message: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push_back(message);
        while self.logs().len() > MAX_LOGS {
            logs.pop_back();
        }
    }
}

impl log::Log for VecLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.logs
                .lock()
                .unwrap()
                .push_back(format!("{}", record.args()));
            while self.logs().len() > MAX_LOGS {
                self.logs.lock().unwrap().pop_front();
            }
        }
    }

    fn flush(&self) {}
}
