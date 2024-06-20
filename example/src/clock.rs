use crate::mutex::Mutex;
use embedded_time::clock::Error;
use embedded_time::duration::{Duration, Fraction};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};
use rp2040_hal::Timer as PicoTimer;

pub struct SystemClock {
    inner: Mutex<Option<PicoTimer>>,
}

impl SystemClock {
    pub const fn default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn initialize(&self, timer: PicoTimer) {
        self.inner.replace(Some(timer))
    }

    /// Returns the current ticks in us since startup
    pub fn get_ticks(&self) -> u64 {
        let mut ticks = 0;

        self.inner.access(|timer| {
            ticks = timer.as_ref().unwrap().get_counter().ticks();
        });

        ticks
    }
}

impl Clock for SystemClock {
    type T = u64;
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        Ok(Instant::new(self.get_ticks()))
    }

    fn new_timer<Dur: Duration>(&self, duration: Dur) -> Timer<OneShot, Armed, Self, Dur>
    where
        Dur: FixedPoint,
    {
        Timer::new(self, duration)
    }
}
