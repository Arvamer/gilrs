GilRs - Game Input Library for Rust
===================================

[![build status](https://gitlab.com/Arvamer/gilrs/badges/master/build.svg)](https://gitlab.com/Arvamer/gilrs/commits/master)

[**Documentation**](https://arvamer.gitlab.io/gilrs/doc/gilrs/)

GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.
Additionally, library is trying to unify different devices, providing single controller layout.

Example
-------

```rust
use gilrs::{Gilrs, Button};

let mut gilrs = Gilrs::new();

// Event loop
loop {
    for (id, event) in gilrs.pool_events() {
        println!("New event from {}: {:?}", id, event);
    }

    if gilrs.gamepad(0).is_btn_pressed(Button::South) {
        println!("Name of gamepad 0: {}", gilrs.gamepad(0).name());
    }
    # break;
}
```

Supported features
------------------

|                  | Input | Hotplugging | Mappings | Force feedback |
|------------------|:-----:|:-----------:|:--------:|:--------------:|
| Linux            |   ✓   |      ✓      |     ✓    |        ✓       |
| Windows (XInput) |   ✓   |      ✓      |    n/a   |        ❌      |
| Windows (DInput) |   ❌  |      ❌     |     ❌   |        ❌      |
| OS X             |   ❌  |      ❌     |     ❌   |        ❌      |
| Android          |   ❌  |      ❌     |     ❌   |        ❌      |

Controller layout
-----------------

![Controller layout](https://arvamer.gitlab.io/gilrs/img/controller.svg)
[original image by nicefrog](http://opengameart.org/content/generic-gamepad-template)

Platform specific notes
======================

Linux
-----

On Linux, GilRs read (and write, in case of force feedback) directly from appropriate
`/dev/input/event*` file. This mean that user have to have read and write access to this file.
On most distros it shouldn't be a problem, but if it is, you will have to create udev rule.
