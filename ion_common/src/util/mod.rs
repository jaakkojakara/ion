use std::time::Duration;

pub(crate) mod log;
pub(crate) mod time;

/// A sub-millisecond accurate sleep function.
///
/// It combines normal OS-sleep with spin-sleep by first sleeping
/// normally for as long as it is safe, and then spinning rest of the way
pub fn native_spin_sleep(duration: Duration) {
    use time::Instant;

    fn sleep_1ms_and_report() -> Duration {
        let start_time = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        let end_time = Instant::now();
        end_time - start_time
    }
    let wake_up_instant = Instant::now() + duration;

    let mut estimated_sleep_length = Duration::from_millis(2);
    let mut duration_left = duration;

    // Native sleep as long as it is safe
    while duration_left > estimated_sleep_length * 2 {
        let current_loop_sleep_length = sleep_1ms_and_report();
        estimated_sleep_length = (estimated_sleep_length + current_loop_sleep_length) / 2;
        if current_loop_sleep_length > duration_left {
            // Slept too long, return immediately
            return;
        } else {
            duration_left -= current_loop_sleep_length;
        }
    }

    // Spin sleep rest of the way
    while wake_up_instant > Instant::now() {
        if cfg!(windows) {
            std::thread::yield_now();
        } else {
            std::hint::spin_loop()
        }
    }
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_spin_sleep_is_accurate() {
        use crate::util::native_spin_sleep;
        use std::time::{Duration, Instant};
        for i in 1..8 {
            let start = Instant::now();

            native_spin_sleep(Duration::from_millis(i));

            let end = Instant::now();

            assert!(end - start < Duration::from_micros(i * 1000 + 50));
            assert!(end - start > Duration::from_micros(i * 1000 - 50));
        }
    }
}
