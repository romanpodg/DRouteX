use super::{AdmissionCode as Code, AdmissionError as Error, Result};
use droutex_config::MAX_GRANT_TIMESTAMP;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const UNCERTAINTY: Duration = Duration::from_secs(2);
#[derive(Clone, Copy)]
pub(super) struct Reading {
    pub wall: Duration,
    pub elapsed: Duration,
}
#[derive(Clone, Copy)]
pub(super) struct Interval {
    pub lower: Duration,
    pub upper: Duration,
    pub elapsed: Duration,
}
impl Interval {
    pub fn floor(self) -> Result<u64> {
        let ceil = self
            .upper
            .as_secs()
            .checked_add(u64::from(self.upper.subsec_nanos() != 0))
            .and_then(|n| n.checked_add(4))
            .map(|n| n / 5 * 5)
            .ok_or(Error(Code::Clock))?;
        if ceil > MAX_GRANT_TIMESTAMP {
            return Err(Error(Code::Clock));
        }
        Ok(ceil)
    }
}
pub(super) struct Clock {
    origin: Instant,
    anchor: Option<Reading>,
    last_elapsed: Option<Duration>,
    last_upper: Option<Duration>,
    faulted: bool,
    recovery_floor: Option<u64>,
    #[cfg(test)]
    pub fake: Option<Reading>,
}
impl Clock {
    pub fn new(recovery_floor: Option<u64>) -> Self {
        let mut value = Self {
            origin: Instant::now(),
            anchor: None,
            last_elapsed: None,
            last_upper: None,
            faulted: false,
            recovery_floor,
            #[cfg(test)]
            fake: None,
        };
        value.anchor = value.read().ok();
        value
    }
    fn read(&self) -> Result<Reading> {
        #[cfg(test)]
        if let Some(reading) = self.fake {
            return Ok(reading);
        }
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error(Code::Clock))?;
        if wall > Duration::from_secs(MAX_GRANT_TIMESTAMP) {
            return Err(Error(Code::Clock));
        }
        Ok(Reading {
            wall,
            elapsed: self.origin.elapsed(),
        })
    }
    pub fn elapsed(&self) -> Duration {
        #[cfg(test)]
        if let Some(reading) = self.fake {
            return reading.elapsed;
        }
        self.origin.elapsed()
    }
    #[cfg(test)]
    pub fn fake_start(&mut self, wall: u64) {
        let reading = Reading {
            wall: Duration::from_secs(wall),
            elapsed: Duration::ZERO,
        };
        self.fake = Some(reading);
        self.anchor = Some(reading);
        self.last_elapsed = None;
        self.last_upper = None;
    }
    pub fn sample(&mut self) -> Result<Interval> {
        if self.faulted {
            return Err(Error(Code::Clock));
        }
        let result = self.interval();
        if result.is_err_and(|e| e.0 == Code::Clock) {
            self.faulted = true;
        }
        result
    }
    fn interval(&mut self) -> Result<Interval> {
        let reading = self.read()?;
        if self.last_elapsed.is_some_and(|last| reading.elapsed < last) {
            return Err(Error(Code::Clock));
        }
        let anchor = self.anchor.ok_or(Error(Code::Clock))?;
        let elapsed = reading
            .elapsed
            .checked_sub(anchor.elapsed)
            .ok_or(Error(Code::Clock))?;
        let predicted = anchor.wall.checked_add(elapsed).ok_or(Error(Code::Clock))?;
        let difference = reading.wall.abs_diff(predicted);
        if difference > UNCERTAINTY {
            return Err(Error(Code::Clock));
        }
        let lower = reading
            .wall
            .min(predicted)
            .checked_sub(UNCERTAINTY)
            .ok_or(Error(Code::Clock))?;
        let upper = reading
            .wall
            .max(predicted)
            .checked_add(UNCERTAINTY)
            .ok_or(Error(Code::Clock))?
            .max(self.last_upper.unwrap_or(Duration::ZERO));
        if upper > Duration::from_secs(MAX_GRANT_TIMESTAMP) {
            return Err(Error(Code::Clock));
        }
        // A legal wall-clock slew must never make an expired lease valid again.
        // Track readings even during the retryable conservative restart wait.
        self.last_elapsed = Some(reading.elapsed);
        self.last_upper = Some(upper);
        if self
            .recovery_floor
            .is_some_and(|floor| lower < Duration::from_secs(floor))
        {
            return Err(Error(Code::RestartWait));
        }
        Ok(Interval {
            lower,
            upper,
            elapsed: reading.elapsed,
        })
    }
    /// Explicit trusted local repair; never lowers the persisted floor.
    pub fn reanchor(&mut self, floor: u64) -> Result<()> {
        self.faulted = true;
        let reading = self.read()?;
        if self.last_elapsed.is_some_and(|last| reading.elapsed < last) {
            return Err(Error(Code::Clock));
        }
        let lower = reading
            .wall
            .checked_sub(UNCERTAINTY)
            .ok_or(Error(Code::Clock))?;
        let floor = floor.max(self.recovery_floor.unwrap_or(0));
        if lower < Duration::from_secs(floor).max(self.last_upper.unwrap_or(Duration::ZERO)) {
            return Err(Error(Code::RestartWait));
        }
        self.anchor = Some(reading);
        self.last_elapsed = Some(reading.elapsed);
        self.recovery_floor = Some(floor);
        self.faulted = false;
        Ok(())
    }
}
