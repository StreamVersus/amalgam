// using smoothed capped delta algoritm

use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Delta {
    last_frame: Instant,
    pub desired_delta: f64,
    smoothed_delta: f64,
    max_delta: f64,
    smoothing_factor: f64,
    min_delta: f64,
    next_frame: Instant,
    should_cap: bool,
}

impl Delta {
    pub fn new(target_fps: f64, min_fps: f64, smoothing_factor: f64, should_cap: bool) -> Self {
        let rev = 1.0 / target_fps;
        Self {
            last_frame: Instant::now(),
            desired_delta: rev,
            smoothed_delta: rev,
            max_delta: 1.0 / min_fps,
            smoothing_factor: smoothing_factor.clamp(0.0, 1.0),
            min_delta: rev,
            next_frame: Instant::now(),
            should_cap,
        }
    }

    pub fn tick(&mut self) -> f64 {
        let raw_delta = self.last_frame.elapsed().as_secs_f64();

        let mut capped_delta = raw_delta
            .min(self.max_delta);

        if self.should_cap {
            capped_delta = capped_delta.max(self.min_delta)
        }
        
        self.smoothed_delta = self.smoothed_delta * self.smoothing_factor
            + capped_delta * (1.0 - self.smoothing_factor);

        self.last_frame = Instant::now();
        self.next_frame = self.last_frame + Duration::from_secs_f64(self.desired_delta);

        self.smoothed_delta
    }
    
    pub fn sleep_till_next_frame(&mut self) {
        if !self.should_cap {
            return;
        }
        
        let dur = self.next_frame.saturating_duration_since(Instant::now());
        thread::sleep(dur);
    }
}

impl Default for Delta {
    fn default() -> Self {
        Delta::new(144.0, 24.0, 0.7, false)
    }
}

impl Delta {
    pub fn set_last_frame(&mut self, last_frame: Instant) {
        self.last_frame = last_frame;
    }

    pub fn set_smoothed_delta(&mut self, smoothed_delta: f64) {
        self.smoothed_delta = smoothed_delta;
    }

    pub fn set_max_delta(&mut self, max_delta: f64) {
        self.max_delta = max_delta;
    }

    pub fn set_smoothing_factor(&mut self, smoothing_factor: f64) {
        self.smoothing_factor = smoothing_factor;
    }

    pub fn set_min_delta(&mut self, min_delta: f64) {
        self.min_delta = min_delta;
    }

    pub fn set_should_cap(&mut self, should_cap: bool) {
        self.should_cap = should_cap;
    }
}