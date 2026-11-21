//! mulberry32, identical to Brickpress 1.x, so stored seeds keep their meaning.

#[derive(Clone, Debug)]
pub struct SeededRandom {
    value: u32,
}

impl SeededRandom {
    pub fn new(seed: i64) -> Self {
        Self { value: seed as u32 }
    }
    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.value = self.value.wrapping_add(0x6d2b_79f5);
        let mut t = self.value;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (t ^ (t >> 14)) as f64 / 4_294_967_296.0
    }
}

impl Iterator for SeededRandom {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        Some(self.next_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeatable_and_distinct() {
        let a: Vec<f64> = SeededRandom::new(1).take(20).collect();
        let b: Vec<f64> = SeededRandom::new(1).take(20).collect();
        let c: Vec<f64> = SeededRandom::new(2).take(20).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
        // Reference value from the JavaScript implementation: seededRandom(1)().
        assert!((a[0] - 0.6270739405881613).abs() < 1e-15, "{}", a[0]);
    }
}
