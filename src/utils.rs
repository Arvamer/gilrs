// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
pub fn apply_deadzone(x: f32, y: f32, threshold: f32) -> (f32, f32) {
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= threshold {
        (0.0, 0.0)
    } else {
        let norm = ((magnitude - threshold) / (1.0 - threshold)) / magnitude;
        (x * norm, y * norm)
    }
}
