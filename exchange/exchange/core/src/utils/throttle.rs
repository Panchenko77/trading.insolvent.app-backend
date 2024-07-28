use std::time;

/// `SimpleThrottler` is a struct that has a number of operations allowed per
/// time interval and the length of the inverval. It also stores the number of
/// operations left to do in the current iterval, and when did the current
/// interval start.
#[derive(Debug)]
pub struct SimpleThrottler {
    times: u64,
    interval: time::Duration,

    times_left: u64,
    last_interval: time::Instant,
}

impl SimpleThrottler {
    pub fn new(operations_per_interval: u64, time_interval: time::Duration) -> SimpleThrottler {
        SimpleThrottler {
            times: operations_per_interval,
            interval: time_interval,

            times_left: operations_per_interval,
            last_interval: time::Instant::now(),
        }
    }

    pub async fn wait(&mut self) {
        let curr_interval = time::Instant::now().duration_since(self.last_interval);
        if curr_interval > self.interval {
            self.reset();
        } else if self.times_left == 0 {
            tokio::time::sleep(self.interval - curr_interval).await;
            self.reset();
        }
        self.times_left -= 1;
    }
    pub fn should_throttle(&mut self) -> bool {
        let curr_interval = time::Instant::now().duration_since(self.last_interval);
        if curr_interval > self.interval {
            self.reset();
            return false;
        }
        self.times_left == 0
    }

    pub fn reset(&mut self) {
        self.times_left = self.times;
        self.last_interval = time::Instant::now()
    }
}
