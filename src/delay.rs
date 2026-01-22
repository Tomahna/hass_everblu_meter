use std::{thread::sleep, time::Duration};

pub struct Timeout{
    pub time: u64,
    pub timeout: u64,
}

impl Timeout {
    pub fn new(timeout: u64) -> Self {
        Timeout{
            time: 0,
            timeout: timeout 
        }
    }

    pub fn delay(&mut self, millis: u64) {
        sleep(Duration::from_millis(millis));
        self.time += millis
    }

    pub fn has_timed_out(&self) -> bool {
        self.time > self.timeout
    }
}
