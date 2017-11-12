Change Log
==========

v0.6.0 - unreleased
-------------------

### Changed

- Renamed `Filter::filter` to `Filter::filter_ev` because RFC 2124 added
  `filter` method to `Option` (our `Filter` is implemented for `Option<Event>`).

### Fixed

- Linux: Fixed axis value normalization if neither minimal value is 0 nor
  midpoint is 0.
- Linux: Ensure that axis values are clamped after normalization.

v0.5.0 - 2017-09-24
-------------------

### Added

- `Mapping::remove_button()` and `Mapping::remove_axis()`.
- `GilrsBuilder` for customizing how `Gilrs` is created.
- Event filters. See `ev::filter` module for more info.
- `Gilrs::next_event()` - use it with `while let` loop in your event loop.
  This allow to avoid borrow checker problems that `EventIterator` caused.
- New event – `Dropped`. Used by filters to indicate that you should ignore
  this event.
- New event – `ButtonRepeated`. Can be emitted by `Repeat` filter.
- `Axis::{DPadX, DPadY}`
- `Gamepad::{button_name, axis_name, button_code, axis_code}` functions for
  accessing mapping data.
- `Gamepad::axis_data, button_data` – part of new extended gamepad state.
- `Gamepad::id()` – returns gamepad ID.
- `Gilrs::update, inc, counter, reset_counter` – part of new extended
   gamepad state.

### Removed

- `Gilrs::with_mappings()` – use `GilrsBuilder`.
- `Gilrs::poll_events()` and `EventIterator` – use `Gilrs::next_event()`
  instead.

### Changed

- Minimal rust version is now 1.19
- New gamepad state. Now can store state for any button or axis (previously was
  only useful for named buttons and axes). Additionally it now also know when
  last event happened. Basic usage with `is_pressed()` and `value()` methods is
  same, but check out documentation for new features.
- Gamepad state now must be explicitly updated with `Gilrs::update(Event)`.
  This change was necessary because filters can change events.
- `Event` is now a struct and contains common information like id of gamepad
  and timestamp (new). Old enum was renamed to `EventType` and can be accessed
  from `Event.event` public field.
- New force feedback module, including support for Windows. There are to many
  changes to list them all here, so pleas check documentation and examples.
- Renamed `ff::Error::EffectNotSupported` to `ff::Error::NotSupported`.
- `Button::Unknown` and `Axis::Unknown` have now value of 0.
- `Gamepad::set_mapping()` (and `_strict` variant) now returns error when
  creating mapping with `Button::Unknown` or `Axis::Unknown`. Additionally
  `_strict` version does not allow `Button::{C, Z}` and Axis::{LeftZ, RightZ}.
- xinput: New values for `NativEvCode`

### Fixed

- Panic on `unreachable!()` when creating mapping with `Button::{C, Z,
  Unknown}` or `Axis::{LeftZ, RightZ}`.

v0.4.4 — 2017-06-16
-------------------

### Changed

- Gilrs no longer uses `ioctl` crate on Linux. Because `ioctl` was deprecated
  and all versions yanked, it was causing problems for new builds that didn't
  have `ioctl` crate listed in Cargo.lock.

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
