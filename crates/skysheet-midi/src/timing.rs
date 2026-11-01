use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use skysheet_score::MAX_TICK;

/// Exact positive finite binary64 as a ratio, including subnormals.
pub(crate) fn dyadic(value: f64) -> Result<(BigUint, BigUint), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err("reference tempo must be finite and positive".into());
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, power) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    let mut numerator = BigUint::from(significand);
    let mut denominator = BigUint::one();
    if power >= 0 {
        numerator <<= power as usize;
    } else {
        denominator <<= (-power) as usize;
    }
    Ok((numerator, denominator))
}

pub(crate) fn round_ratio(n: &BigUint, d: &BigUint) -> (BigUint, bool) {
    debug_assert!(!d.is_zero());
    let q = n / d;
    let r = n % d;
    let fractional = !r.is_zero();
    (q + BigUint::from(u8::from((&r << 1) >= *d)), fractional)
}

pub(crate) struct Timing {
    // Absolute weighted microsecond-ticks at each tempo boundary.
    segments: Vec<(u64, BigUint, u32)>,
    numerator: BigUint,
    denominator: BigUint,
}

impl Timing {
    pub(crate) fn new(ppq: u16, tempos: &[(u64, u32)], bpm: f64) -> Result<Self, String> {
        if ppq == 0 || tempos.first().map(|t| t.0) != Some(0) {
            return Err("invalid timing map origin or PPQ".into());
        }
        let (bn, bd) = dyadic(bpm)?;
        let mut segments = Vec::new();
        let mut weight = BigUint::zero();
        let mut last_tick = 0;
        let mut last_tempo = 0;
        for &(tick, tempo) in tempos {
            if tempo == 0 || tick < last_tick {
                return Err("invalid source tempo map".into());
            }
            weight += BigUint::from(tick - last_tick) * last_tempo;
            segments.push((tick, weight.clone(), tempo));
            last_tick = tick;
            last_tempo = tempo;
        }
        Ok(Self {
            segments,
            numerator: bn * 960_u32,
            denominator: bd * u32::from(ppq) * 60_000_000_u32,
        })
    }

    pub(crate) fn endpoint(&self, tick: u64) -> Result<(u64, bool), String> {
        let index = self.segments.partition_point(|s| s.0 <= tick) - 1;
        let (origin, weight, tempo) = &self.segments[index];
        let n = (weight + BigUint::from(tick - origin) * *tempo) * &self.numerator;
        let (rounded, fractional) = round_ratio(&n, &self.denominator);
        if rounded > BigUint::from(MAX_TICK) {
            return Err(format!(
                "rounded endpoint at source tick {tick} exceeds Score MAX_TICK"
            ));
        }
        Ok((
            rounded.to_u64().ok_or("endpoint narrowing failed")?,
            fractional,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_fixed_width_reference_for_sampled_exact_bpm_and_maps() {
        // Independent bounded-u128 formula; expected endpoints do not call the
        // production dyadic decoder or quantizer. BPM values are exact k/8.
        for p in [1_u16, 24, 480, 960, 32767] {
            for k in [1_u32, 479, 960, 1001, 65535] {
                let timing = Timing::new(
                    p,
                    &[(0, 500000), (1000, 666667), (2000, 1)],
                    f64::from(k) / 8.0,
                )
                .unwrap();
                for tick in [0_u64, 1, 999, 1000, 1001, 1999, 2000, 2001, 1_000_000_000] {
                    let weight = u128::from(tick.min(1000)) * 500000
                        + u128::from(tick.saturating_sub(1000).min(1000)) * 666667
                        + u128::from(tick.saturating_sub(2000));
                    let n = weight * 960 * u128::from(k);
                    let d = u128::from(p) * 60_000_000 * 8;
                    let expected = (2 * n + d) / (2 * d);
                    assert_eq!(
                        timing.endpoint(tick).unwrap(),
                        (expected as u64, n % d != 0)
                    );
                }
            }
        }
    }

    #[test]
    fn default_source_bpm_is_nearest_binary64_with_exact_interval_certificate() {
        // Division operands are exact integers. Independently certify its result
        // against neighboring binary64 midpoints for source tempo samples.
        fn ratio(bits: u64) -> (BigUint, BigUint) {
            let power = ((bits >> 52) & 2047) as i32 - 1075;
            let sig = BigUint::from((bits & ((1_u64 << 52) - 1)) | (1_u64 << 52));
            if power >= 0 {
                (sig << power as usize, 1_u8.into())
            } else {
                (sig, BigUint::one() << (-power) as usize)
            }
        }
        for us in [
            1_u32, 3, 7, 499999, 500000, 500001, 600001, 1000000, 0xffffff,
        ]
        .into_iter()
        .chain((1..=1000).map(|i| i * 16777))
        {
            let bits = (60_000_000.0 / f64::from(us)).to_bits();
            let (n, d) = ratio(bits);
            for (neighbor, lower) in [(bits - 1, true), (bits + 1, false)] {
                let (nn, dd) = ratio(neighbor);
                let left = BigUint::from(60_000_000_u32) * 2_u32 * &d * &dd;
                let right = (n.clone() * dd + nn * &d) * us;
                let valid = if lower { left > right } else { left < right };
                assert!(valid || (left == right && bits & 1 == 0));
            }
        }
    }

    #[test]
    fn d6_exact_rounding_at_limit_and_half_neighbors() {
        let halves = Timing::new(1920, &[(0, 500000)], 120.0).unwrap();
        assert_eq!(halves.endpoint(MAX_TICK * 2).unwrap(), (MAX_TICK, false));
        assert!(halves.endpoint(MAX_TICK * 2 + 1).is_err());
        // Exact hundredth grid via source tempo 5000 and reference BPM 120.
        let hundredths = Timing::new(960, &[(0, 5000)], 120.0).unwrap();
        assert_eq!(
            hundredths.endpoint(MAX_TICK * 100 + 49).unwrap(),
            (MAX_TICK, true)
        );
        assert!(hundredths.endpoint(MAX_TICK * 100 + 50).is_err());
        for (n, d, expected) in [
            (0, 2, 0),
            (1, 2, 1),
            (3, 2, 2),
            (200, 3, 67),
            (400, 3, 133),
            (200, 1, 200),
            (MAX_TICK, 1, MAX_TICK),
        ] {
            assert_eq!(
                round_ratio(&n.into(), &BigUint::from(d as u64)).0,
                expected.into()
            );
        }
        let m = BigUint::from(MAX_TICK);
        assert_eq!(round_ratio(&(&m * 100_u32 + 49_u32), &100_u32.into()).0, m);
        assert_eq!(
            round_ratio(&(&m * 2_u32 + 1_u32), &2_u32.into()).0,
            &m + 1_u32
        );
    }

    #[test]
    fn d6_absolute_endpoints_and_extreme_tempos() {
        let t = Timing::new(960, &[(0, 500000), (960, 1000000)], 120.0).unwrap();
        assert_eq!(t.endpoint(480).unwrap(), (480, false));
        assert_eq!(t.endpoint(1440).unwrap(), (1920, false));
        assert_eq!(t.endpoint(1920).unwrap(), (2880, false));
        let t = Timing::new(960, &[(0, 500000), (960, 1000000)], 60.0).unwrap();
        assert_eq!(t.endpoint(1920).unwrap().0, 1440);
        assert_eq!(
            Timing::new(960, &[(0, 500000)], 120.0)
                .unwrap()
                .endpoint(MAX_TICK)
                .unwrap(),
            (MAX_TICK, false)
        );
        assert_eq!(
            Timing::new(960, &[(0, 500000)], f64::from_bits(1))
                .unwrap()
                .endpoint(960)
                .unwrap(),
            (0, true)
        );
        assert!(
            Timing::new(960, &[(0, 500000)], f64::MAX)
                .unwrap()
                .endpoint(960)
                .is_err()
        );
        assert!(dyadic(0.0).is_err());
        assert!(dyadic(f64::INFINITY).is_err());
        assert!(dyadic(f64::NAN).is_err());
    }
}
