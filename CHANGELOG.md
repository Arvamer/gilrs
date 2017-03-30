Change Log
==========

v0.5.0 - unreleased
-------------------

### Added

- New type of force feedback effect—rumble (see `ff::EffectType::Rumble`).
- New variants in `ff::Error` enum—`ff::Error::{Disconnected, InvalidId, Other}`.

### Changed

- Redesigned `ff::EffectData` to allow other effect types.
- Renamed `ff::Error::EffectNotSupported` to `ff::Error::NotSupported`.
- Improved error handling in force feedback related functions.
  `Gamepad::{drop_ff_effect, set_ff_gain, max_ff_effects}` and
  `ff::Effect::stop` now return `Result`
  
### Removed

- Removed `ff:Trigger`—usually you want to play force feedback effect when some event happen in game,
  not when button is pressed. Additionally XInput  does not support it natively. If you used this
  feature, pleas open an issue.

v0.4.3 — 2017-03-12
-------------------

### Added

- You can now iterate over mutable references to connected gamepads using
  `Gilrs::gamepads_mut()`.

### Fixed

- Fixed `unreachable!()` panic on 32bit Linux
- Improved converting axes values to `f32` when using XInput

v0.4.2 - 2017-01-15
-------------------

### Changed

- Updated SDL_GameControllerDB to latest revision.
- Changes in axes values that are less than 1% are now ignored.

### Fixed

- Fixed multiple axes mapped to same axis name when mappings are incomplete.
- Values returned with `AxisChanged` event now have correctly applied
  deadzones.
- Linux: Correctly handle event queue overrun.


v0.4.1 - 2016-12-12
-------------------

### Fixed

- Type inference error introduced by generic index in `<[T]>::get`

v0.4.0 - 2016-12-11
-------------------

### Added

- `Gamepad::mappings_source(&self)` which can be used to filter gamepads which
  not provide unified controller layout
- `MappingsSource` enum
- You can now set custom mapping for gamepad with `Gamepad::set_mapping(…)`
- `Gilrs::with_mappings(&str)` to create Gilrs with additional gamepad mappings

### Changed

- Button and axis events now also have native event codes
- On Linux, if button or axis is not known, is now reported as `Unknown`
  (previously all unknown events have been ignored)
- More devices are now treated as gamepads on Linux (use `mappings_source()` to
  filter unwanted gamepads)
- Renamed `{Gamepad,GamepadState}::is_btn_pressed(Button)` to
  `is_pressed(Button)`
- Renamed `{Gamepad,GamepadState}::axis_val(Axis)` to `value(Axis)`

### Fixed

- Integer overflow if button with keyboard code was pressed on Linux
- `Gilrs` should no longer panic if there are some unexpected problems with
  Udev
- Fixed normalization of axes values on Linux

v0.3.1 - 2016-09-23
-------------------

### Fixed

- Fixed compilation error on non-x86_64 Linux

v0.3.0 - 2016-09-22
-------------------

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
- `Effect::play(&self, u16)` now returns `Result<(), Error>`
- Linux: Reduced memory usage

### Removed

- `ff::Direction` no longer implements `From<f32>`

### Fixed

- Buttons west and east are no longer swapped when using SDL2 mappings
- Linux: infinite loop after gamepad disconnects
- Linux: SDL2 mappings for gamepads that can also report mouse and keyboard
  events now should works

v0.2.0 - 2016-08-18
------

### Changed

- Rename `Gilrs::pool_events()` to `Gilrs::poll_events()`

### Fixed

- Linux: Disconnected events are now emitted properly
- Linux: All force feedback effects are now dropped when gamepad disconnects
