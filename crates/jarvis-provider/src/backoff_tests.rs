use std::time::Duration;

use super::{FixedJitter, Jitter, RandomJitter, base_delay, delay_for};

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

#[test]
fn base_delay_doubles_from_half_a_second_up_to_the_cap() {
    let expected = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 32.0, 32.0];
    for (index, want) in expected.into_iter().enumerate() {
        let failed = u32::try_from(index).unwrap_or(0) + 1;
        assert_eq!(base_delay(failed), secs(want), "başarısız deneme {failed}");
    }
    assert_eq!(base_delay(0), secs(0.5), "0, 1 sayılır");
    assert_eq!(base_delay(u32::MAX), secs(32.0), "taşma yok");
}

struct Fixed(Duration);

impl Jitter for Fixed {
    fn jitter(&self, _max: Duration) -> Duration {
        self.0
    }
}

struct Full;

impl Jitter for Full {
    fn jitter(&self, max: Duration) -> Duration {
        max
    }
}

#[test]
fn delay_adds_jitter_of_at_most_a_quarter_of_the_base() {
    assert_eq!(delay_for(2, &FixedJitter::NONE), secs(1.0));
    assert_eq!(
        delay_for(2, &Full),
        secs(1.25),
        "jitter üst sınırı tabanın 1/4'ü"
    );
    assert_eq!(delay_for(7, &Full), secs(40.0));
}

#[test]
fn a_misbehaving_jitter_source_cannot_exceed_the_bound() {
    let huge = Fixed(Duration::from_hours(1));
    assert_eq!(delay_for(1, &huge), secs(0.625));
}

#[test]
fn random_jitter_stays_in_range_and_varies() {
    let jitter = RandomJitter;
    let max = Duration::from_millis(250);
    let draws: Vec<Duration> = (0..64).map(|_| jitter.jitter(max)).collect();
    assert!(draws.iter().all(|d| *d <= max), "{draws:?}");
    assert!(
        draws.iter().any(|d| *d > Duration::ZERO),
        "hep sıfır: {draws:?}"
    );
    assert!(
        draws.windows(2).any(|w| w.first() != w.get(1)),
        "hep aynı: {draws:?}"
    );
    assert_eq!(jitter.jitter(Duration::ZERO), Duration::ZERO);
}

#[test]
fn fixed_jitter_returns_its_value_capped_at_the_bound() {
    let three = FixedJitter(Duration::from_millis(3));
    assert_eq!(
        three.jitter(Duration::from_millis(10)),
        Duration::from_millis(3)
    );
    assert_eq!(
        three.jitter(Duration::from_millis(2)),
        Duration::from_millis(2)
    );
    assert_eq!(
        FixedJitter::NONE.jitter(Duration::from_secs(1)),
        Duration::ZERO
    );
}
