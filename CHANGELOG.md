Change Log
==========

v0.3.0 - unreleased

### Changed

- Rename `Button::Unknow` to `Button::Unknown`
- `Gamepad::name(&self)` now returns `&str` instead of `&String`

v0.2.0 - 2016-08-18
------

### Changed

- Rename `Gilrs::pool_events()` to `Gilrs::poll_events()`

### Fixed

- linux: Disconnected events are now emitted properly
- linux: All force feedback effects are now dropped when gamepad disconnects
