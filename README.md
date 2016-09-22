GilRs - Game Input Library for Rust
===================================

[![build status](https://gitlab.com/Arvamer/gilrs/badges/master/build.svg)](https://gitlab.com/Arvamer/gilrs/commits/master)
[![Crates.io](https://img.shields.io/crates/v/gilrs.svg)](https://crates.io/crates/gilrs)
![Crates.io](https://img.shields.io/crates/l/gilrs.svg)

[**Documentation (master)**](https://arvamer.gitlab.io/gilrs/doc/gilrs/)

GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.

Main features:

- Unified gamepad layout - buttons and axes are represented by familiar names
- Support for SDL2 mappings including `SDL_GAMECONTROLLERCONFIG` environment
  variable which Steam uses
- Hotplugging - GilRs will try to assign new ID for new gamepads and reuse same
  ID for gamepads which reconnected
- Force feedback (rumble)
- Power information (is gamepad wired, current battery status)

The main repository for project [is on GitLab](https://gitlab.com/Arvamer/gilrs)
although there is also [GitHub mirror](https://github.com/Arvamer/gilrs).
Please use GitLab's issue tracker and merge requests.

This repository contains submodule - after you clone it, don't forget to run
`git submodule init; git submodule update` (or clone with `--recursive` flag)
or you will get compile errors.

Example
-------

```toml
[dependencies]
gilrs = "0.3.0"
```

```rust
use gilrs::{Gilrs, Button};

let mut gilrs = Gilrs::new();

// Event loop
loop {
    for (id, event) in gilrs.poll_events() {
        println!("New event from {}: {:?}", id, event);
    }

    if gilrs.gamepad(0).is_btn_pressed(Button::South) {
        println!("Name of gamepad 0: {}", gilrs.gamepad(0).name());
    }
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

Platform specific notes
======================

Linux
-----

On Linux, GilRs read (and write, in case of force feedback) directly from appropriate
`/dev/input/event*` file. This mean that user have to have read and write access to this file.
On most distros it shouldn't be a problem, but if it is, you will have to create udev rule.

To build GilRs, you will need pkg-config and libudev .pc file. On some
distributions this file is packaged in separate archive (for example `libudev-dev` in Debian).

License
=======

This project is licensed under the terms of both the Apache License (Version 2.0) and the MIT
license. See LICENSE-APACHE and LICENSE-MIT for details.
