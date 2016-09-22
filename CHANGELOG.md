Change Log
==========

v0.3.0 - unreleased

### Added

- `Gamepad::power_info(&self)`
- `ff::Direction::from_radians(f32)` and `ff::Direction::from_vector([f32; 2])`
- `Gilrs::gamepads(&self)` which returns iterator over all connected gamepads
- `GamepadState` now implements `is_btn_pressed(Button)` and `axis_val(Axis)`
- `Gilrs` now implements `Index`and `IndexMut`

### Changed

- Rename `Button::Unknow` to `Button::Unknown`
- `Gamepad::name(&self)` now returns `&str` instead of `&String`
- Improved dead zone detection

### Removed

- `ff::Direction` no longer implements `From<f32>`

### Fixed

- Buttons west and east are no longer swapped when using SDL2 mappings
- Linux: infinite loop after gamepad disconnects

v0.2.0 - 2016-08-18
------

### Changed

- Rename `Gilrs::pool_events()` to `Gilrs::poll_events()`

### Fixed

- Linux: Disconnected events are now emitted properly
- Linux: All force feedback effects are now dropped when gamepad disconnects
