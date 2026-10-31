//! Recover the historical sample stream without an interpreter. The integer
//! seed, MT19937 init-by-array, word assembly and rejection sampling match the
//! documented legacy stream; ordinary runs consume the frozen bit manifest.
use crate::oracle::MAX_BITS;
use std::collections::HashSet;

pub const SEED: u32 = 0x534b59;

pub struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    pub fn seeded(key: &[u32]) -> Self {
        assert!(!key.is_empty());
        let mut s = Self {
            state: [0; 624],
            index: 624,
        };
        s.state[0] = 19650218;
        for i in 1..624 {
            s.state[i] = 1812433253u32
                .wrapping_mul(s.state[i - 1] ^ (s.state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..624.max(key.len()) {
            s.state[i] = (s.state[i]
                ^ (s.state[i - 1] ^ (s.state[i - 1] >> 30)).wrapping_mul(1664525))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                s.state[0] = s.state[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            s.state[i] = (s.state[i]
                ^ (s.state[i - 1] ^ (s.state[i - 1] >> 30)).wrapping_mul(1566083941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                s.state[0] = s.state[623];
                i = 1;
            }
        }
        s.state[0] = 0x80000000;
        s
    }

    pub fn word(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }

    pub fn get_bits(&mut self, count: u32) -> u64 {
        assert!((1..=64).contains(&count));
        if count <= 32 {
            return u64::from(self.word() >> (32 - count));
        }
        let low = u64::from(self.word());
        low | (u64::from(self.word() >> (64 - count)) << 32)
    }
}

pub fn historical() -> Vec<u64> {
    let mut rng = Mt19937::seeded(&[SEED]);
    let mut values = vec![1, 2, (1 << 52) - 1, 1 << 52, MAX_BITS];
    values.extend((0..2047).map(|e| (e << 52) | rng.get_bits(52)));
    let mut seen = HashSet::new();
    values.retain(|v| *v != 0 && seen.insert(*v));
    while values.len() < 10000 {
        let draw = rng.get_bits(63);
        if draw < MAX_BITS && seen.insert(draw + 1) {
            values.push(draw + 1);
        }
    }
    values
}
