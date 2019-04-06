Change Log
==========

v0.2.2 - 2018-04-06
-------------------

### Changed

- Windows: XInput is now dynamically loaded using rusty-xinput

### Fixed

- xinput: incorrect `is_connected()` after hotplugging
- wasm: Incorrect gamepad IDs in `Disconnected` event (@ryanisaacg)

v0.2.1 - 2018-02-25
-------------------

### Fixed

- Compilation error on macOS

v0.2.0 - 2018-02-21
-------------------

### Added

- Initial support for macOS (@jtakakura). There are still some functionality
  missing, check related issues in #58.
- Wasm support, using stdweb (@ryanisaacg).

### Changed

- `AxisInfo::deadzone` is now a `Option`.
- Minimal supported version is now 1.31.1. The crate can still be build with
  older rustc, but it may change during next patch release.

### Removed

- `AxisInfo::deadzone()` function.

### Fixed

- xinput: Incorrect gamepad ID when more than one gamepad is connected (@DTibbs).
