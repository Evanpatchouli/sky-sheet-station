//! Exact interval search, not the production decoder and not a significand
//! normalization/division translation of the former oracle. See the proof in
//! docs/numeric-verification.md. No floating-point operations occur here.
use num_bigint::BigUint;
use num_traits::One;
use std::cmp::Ordering;

pub const MAX_BITS: u64 = 0x7fef_ffff_ffff_ffff;

#[derive(Clone, Debug)]
pub struct Exact {
    pub n: BigUint,
    pub d: BigUint,
}

impl Exact {
    pub fn cmp_exact(&self, other: &Self) -> Ordering {
        (&self.n * &other.d).cmp(&(&other.n * &self.d))
    }

    pub fn midpoint(&self, other: &Self) -> Self {
        Self {
            n: &self.n * &other.d + &other.n * &self.d,
            d: (&self.d * &other.d) << 1usize,
        }
    }

    pub fn decimal_text(&self) -> String {
        // Boundary corpus values are dyadic. Reduce powers of two so the text
        // matches the historical exact Fraction rendering, including trailing 0s.
        let mut n = self.n.clone();
        let mut d = self.d.clone();
        while d > BigUint::one() && !n.bit(0) {
            n >>= 1usize;
            d >>= 1usize;
        }
        let places = (d.bits() - 1) as usize;
        assert_eq!(d, BigUint::one() << places);
        if places == 0 {
            return n.to_string();
        }
        let digits = (n * BigUint::from(5u8).pow(places as u32)).to_string();
        let padded = format!("{:0>width$}", digits, width = places + 1);
        let split = padded.len() - places;
        format!("{}.{}", &padded[..split], &padded[split..])
    }
}

pub fn dyadic(bits: u64) -> Exact {
    assert!(bits <= MAX_BITS);
    let exponent = (bits >> 52) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (significand, shift) = if exponent == 0 {
        (fraction, -1074)
    } else {
        ((1u64 << 52) + fraction, exponent - 1023 - 52)
    };
    if shift >= 0 {
        Exact {
            n: BigUint::from(significand) << shift as usize,
            d: BigUint::one(),
        }
    } else {
        Exact {
            n: BigUint::from(significand),
            d: BigUint::one() << (-shift) as usize,
        }
    }
}

fn overflow_threshold() -> Exact {
    Exact {
        n: (BigUint::one() << 1024usize) - (BigUint::one() << 970usize),
        d: BigUint::one(),
    }
}

enum DecimalValue {
    Nonpositive,
    Outside, // Clearly rounds to zero or infinity, without huge powers of ten.
    Positive(Exact),
}

fn decimal(token: &str) -> Result<DecimalValue, String> {
    if token.len() > 1_000_000 {
        return Err("numeric tool capacity exceeded (not an input-format limit)".into());
    }
    let negative = token.starts_with('-');
    let unsigned = token.strip_prefix('-').unwrap_or(token);
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let mut digits = String::new();
    let mut fractional = 0i64;
    let mut after_dot = false;
    for byte in mantissa.bytes() {
        match byte {
            b'.' if !after_dot => after_dot = true,
            b'0'..=b'9' => {
                digits.push(char::from(byte));
                fractional += i64::from(after_dot);
            }
            _ => return Err("oracle requires a validated JSON numeric token".into()),
        }
    }
    if digits.is_empty() {
        return Err("empty numeric coefficient".into());
    }
    let significant = digits.trim_start_matches('0');
    if significant.is_empty() || negative {
        return Ok(DecimalValue::Nonpositive);
    }
    let scale = exponent
        .parse::<i64>()
        .map_err(|_| "numeric exponent exceeds tool capacity")?
        .checked_sub(fractional)
        .ok_or("numeric exponent exceeds tool capacity")?;
    let order = scale
        .checked_add(significant.len() as i64 - 1)
        .ok_or("numeric exponent exceeds tool capacity")?;
    if !(-324..=308).contains(&order) {
        return Ok(DecimalValue::Outside);
    }
    let coefficient =
        BigUint::parse_bytes(significant.as_bytes(), 10).ok_or("invalid decimal coefficient")?;
    let power =
        u32::try_from(scale.unsigned_abs()).map_err(|_| "numeric scale exceeds tool capacity")?;
    let ten = BigUint::from(10u8).pow(power);
    Ok(DecimalValue::Positive(if scale >= 0 {
        Exact {
            n: coefficient * ten,
            d: BigUint::one(),
        }
    } else {
        Exact {
            n: coefficient,
            d: ten,
        }
    }))
}

pub fn bits(token: &str) -> Result<Option<u64>, String> {
    let DecimalValue::Positive(x) = decimal(token)? else {
        return Ok(None);
    };
    if x.cmp_exact(&overflow_threshold()) != Ordering::Less {
        return Ok(None);
    }
    // Invariant: the greatest representable finite value <= x is in [lo, hi].
    let (mut lo, mut hi) = (0, MAX_BITS);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if dyadic(mid).cmp_exact(&x) != Ordering::Greater {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let chosen = if lo == MAX_BITS {
        lo
    } else {
        match x.cmp_exact(&dyadic(lo).midpoint(&dyadic(lo + 1))) {
            Ordering::Less => lo,
            Ordering::Greater => lo + 1,
            Ordering::Equal => {
                if lo & 1 == 0 {
                    lo
                } else {
                    lo + 1
                }
            }
        }
    };
    let answer = (chosen != 0).then_some(chosen);
    if !certify(token, answer)? {
        return Err("oracle interval certificate failed".into());
    }
    Ok(answer)
}

/// Independent postcondition: check membership in the output's rounding cell.
/// No search, no comparison to a host float parser, and no result from the DUT.
pub fn certify(token: &str, candidate: Option<u64>) -> Result<bool, String> {
    let DecimalValue::Positive(x) = decimal(token)? else {
        return Ok(candidate.is_none());
    };
    let Some(b) = candidate else {
        return Ok(
            x.cmp_exact(&dyadic(0).midpoint(&dyadic(1))) != Ordering::Greater
                || x.cmp_exact(&overflow_threshold()) != Ordering::Less,
        );
    };
    if b == 0 || b > MAX_BITS {
        return Ok(false);
    }
    let lower = dyadic(b - 1).midpoint(&dyadic(b));
    let upper = if b == MAX_BITS {
        overflow_threshold()
    } else {
        dyadic(b).midpoint(&dyadic(b + 1))
    };
    let even = b & 1 == 0;
    let low = x.cmp_exact(&lower);
    let high = x.cmp_exact(&upper);
    Ok(
        (low == Ordering::Greater || (even && low == Ordering::Equal))
            && (high == Ordering::Less || (even && high == Ordering::Equal)),
    )
}
