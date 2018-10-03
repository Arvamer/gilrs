#![allow(dead_code)]

/// Returns true if nth bit in array is 1.
pub fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}
