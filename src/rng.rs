//! A tiny seedable RNG (xorshift64*) so `Strategy::Random` is reproducible in tests
//! without pulling in an external crate.

pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        // xorshift needs a non-zero state
        Xorshift64 { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform value in `0..bound`. Bound must be non-zero.
    pub fn next_below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Xorshift64::new(42);
        let mut b = Xorshift64::new(42);
        for _ in 0..20 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seed_diverges() {
        let mut a = Xorshift64::new(1);
        let mut b = Xorshift64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }
}
