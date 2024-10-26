use std::time::{Duration, SystemTime};

pub(crate) struct SmartUpdate<AppState>
where
    AppState: Eq,
{
    last_state: AppState,
    last_change_time: SystemTime,
    // Minimum time between updates, in ms, e.g. 200.
    freq_ms: u32,
}

impl<AppState> SmartUpdate<AppState>
where
    AppState: Eq,
{
    pub fn new(freq_ms: u32, init_state: AppState) -> Self {
        Self {
            last_state: init_state,
            last_change_time: SystemTime::now() - Duration::from_secs(3600),
            freq_ms,
        }
    }

    /// Return if external update is needed
    pub fn update_state(&mut self, state: AppState) -> bool {
        if state == self.last_state {
            // state did not change
            false
        } else {
            let now = SystemTime::now();
            let time_diff_ms = now
                .duration_since(self.last_change_time)
                .expect("Time went backwards")
                .as_millis();
            // println!("{:?}", time_diff_ms);
            let need_update = time_diff_ms >= self.freq_ms as u128;
            // update state
            self.last_state = state;
            if need_update {
                self.last_change_time = now;
            }
            need_update
            // TODO: place time if no need for update now
        }
    }
}
