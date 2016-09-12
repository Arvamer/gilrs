Change Log
==========

v0.3.0 - unreleased

### Added

- `Gamepad::power_info(&self)`

### Changed

- Rename `Button::Unknow` to `Button::Unknown`
- `Gamepad::name(&self)` now returns `&str` instead of `&String`

### Fixed

- Linux: infinite loop after gamepad disconnects

v0.2.0 - 2016-08-18
------

### Changed

- Rename `Gilrs::pool_events()` to `Gilrs::poll_events()`

### Fixed

- Linux: Disconnected events are now emitted properly
- Linux: All force feedback effects are now dropped when gamepad disconnects
