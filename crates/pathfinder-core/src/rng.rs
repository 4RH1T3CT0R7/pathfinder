/// Xoshiro256** seedable PRNG for reproducible maze generation.
pub struct Xoshiro256 {
    state: [u64; 4],
}

impl Xoshiro256 {
    pub fn new(seed: u64) -> Self {
        // Use SplitMix64 to expand a single seed into 4 state words
        let mut sm = SplitMix64(seed);
        let state = [sm.next(), sm.next(), sm.next(), sm.next()];
        Xoshiro256 { state }
    }

    /// Returns a random u64.
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.state[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];

        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    /// Returns a random u32.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Returns a random number in [0, bound).
    pub fn next_bound(&mut self, bound: u32) -> u32 {
        // Lemire's nearly divisionless method
        let mut x = self.next_u32();
        let mut m = (x as u64) * (bound as u64);
        let mut l = m as u32;
        if l < bound {
            let t = bound.wrapping_neg() % bound;
            while l < t {
                x = self.next_u32();
                m = (x as u64) * (bound as u64);
                l = m as u32;
            }
        }
        (m >> 32) as u32
    }

    /// Shuffle a slice in-place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.next_bound((i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let mut rng1 = Xoshiro256::new(42);
        let mut rng2 = Xoshiro256::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_different_seeds() {
        let mut rng1 = Xoshiro256::new(1);
        let mut rng2 = Xoshiro256::new(2);
        let mut same = true;
        for _ in 0..10 {
            if rng1.next_u64() != rng2.next_u64() {
                same = false;
                break;
            }
        }
        assert!(!same);
    }

    #[test]
    fn test_bound() {
        let mut rng = Xoshiro256::new(123);
        for _ in 0..1000 {
            let val = rng.next_bound(10);
            assert!(val < 10);
        }
    }

    #[test]
    fn test_shuffle() {
        let mut rng = Xoshiro256::new(99);
        let mut arr: Vec<u32> = (0..20).collect();
        let original = arr.clone();
        rng.shuffle(&mut arr);
        // Very unlikely to stay identical after shuffle
        assert_ne!(arr, original);
        // But contains the same elements
        arr.sort();
        assert_eq!(arr, original);
    }
}
