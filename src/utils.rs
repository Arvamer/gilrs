// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(dead_code)]

/// Returns true if nth bit in array is 1.
pub fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

/// Like `(a: f32 / b).ceil()` but for integers.
pub fn ceil_div(a: u32, b: u32) -> u32 {
    if a == 0 {
        0
    } else {
        1 + ((a - 1) / b)
    }
}

pub fn clamp(x: f32, min: f32, max: f32) -> f32 {
    x.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_test_bit() {
        let buf = [0b1001_0001u8, 0b0010_0001];
        assert_eq!(test_bit(0, &buf), true);
        assert_eq!(test_bit(3, &buf), false);
        assert_eq!(test_bit(7, &buf), true);
        assert_eq!(test_bit(8, &buf), true);
        assert_eq!(test_bit(15, &buf), false);
    }

    #[test]
    fn t_clamp() {
        assert_eq!(clamp(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp(2.0, 0.0, 1.0), 1.0);
    }
}
