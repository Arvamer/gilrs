Change Log
==========

v0.5.12 - 2024-06-15
----------

### Fixed

- Fixed building on FreeBSD and DragonFly by not using linux implementation

### Changed

- Updated dependencies

v0.5.11 - 2024-03-06
----------

### Added

- Added `vendor_id()` and `product_id()` to `Gamepad`.

### Changed

- Updated `windows` crate to 0.54.

v0.5.10 - 2023-12-17
----------

### Changed

- Updated `windows` crate to 0.52.

v0.5.9 - 2023-11-13
----------

### Fixed

- Disabled unnecessary default features for `inotify`.

v0.5.8 - 2023-11-11
----------

### Added

- Flatpak is now supported by using inotify instead of udev. (!104)

### Changed

- All thread spawned by gilrs are now named. (!102)
- MSRV is now 1.65.

### Fixed

- Linux: Fixed delay in Gilrs::new by limiting udev scan to the input
  subsystem. (!101)

### Fixed

v0.5.7 - 2023-08-22
----------

### Fixed

- windows: Join wgi thread on `Gilrs`'s drop
- wasm: Fix trigger2 only sending binary values

## Changed

- Update `windows` to 0.51

v0.5.6 - 2023-06-19
----------

### Fixed

- Linux: fixed panic when calling `get_power_info` on disconnected gamepad.

v0.5.5 - 2023-04-23
----------

### Added

- `Gilrs::next_event_blocking()`

v0.5.4 - 2023-04-03
----------

### Changed

- Updated `io-kit-sys`, `windows` and `nix`

v0.5.3 - 2023-03-29
----------

### Changed

- Updated `windows` to 0.44

### Fixed

- web: Fixed handling of disconnected gamepads

v0.5.2 - 2022-12-16
----------

### Changed

- `Gilrs` is now `Send` on Linux.

### Fixed

- Crash when app is launched through steam on Windows (see
  https://github.com/microsoft/windows-rs/issues/2252 for details).

v0.5.1 - 2022-11-13
-------------------

### Fixed

- macOS: Fixed that hat axes were sometimes added before other axes breaking
  SDL mappings.
- web: Fixed swapped north and west buttons for gamepads with "standard"
  mapping

v0.5.0 - 2022-11-06
--------------------

### Changed

- Windows now defaults to using Windows Gaming Input instead of xinput.

  If you need to use xInput you can disable the `wgi` feature (It's enabled by
  default) and enable the `xinput` feature.
  ``` toml
  gilrs-core = {version = "0.5.0", default-features = false, features = ["wgi"]}
  ```
- Apps on Windows will now require a focused window to receive inputs by
  default.

  This is a limitation of Windows Gaming Input. It requires an in focus Window
  be associated with the process to receive events. You can still switch back
  to using xInput by turning off default features and enabling the `xinput`
  feature.

- Minimal supported rust version is now 1.64.

### Fixed

- `Gamepad::axes()` on macos now also returns "hat" axes. This should fix dpad
  on single Switch Joy-Con.

v0.4.1 - 2022-05-29
-------------------

### Changed

- Updated io-kit-sys to 0.2 and core-foundation to 0.9 (@jtakakura).
- Reduced numer of enabled features for nix crate (@rtzoeller).

v0.4.0 - 2022-05-22
-------------------

### Changed

- wasm: web-sys/wasm-bindgen is now used by default, dependency on stdweb
  and `wasm-bindgen` feature are removed.
- Minimal supported rust version is now 1.56.
- Updated `uuid` and `nix` to current version.

### Fixed

- wasm: `next_event()` no longer panic if `getGamepads()` is not available.

v0.3.2 - 2021-12-30
-------------------

### Changed

- Updated dependencies

v0.3.1 - 2021-03-30
-------------------

### Added

- Add support for wasm-bindgen (@coolreader18)

v0.3.0 - 2020-10-09
-------------------

### Added

- macos: dpad is supported as a set of dpad axes (gilrs filters dpad axes to
  dpad buttons) (@cleancut).

### Changed

- Minimal supported version is now 1.40

v0.2.6 - 2020-05-11
-------------------

Fixed compilation on musl.

v0.2.5 - 2019-11-30
-------------------

Updated dependencies.

v0.2.4 - 2019-09-05
-------------------

### Fixed

- Fixed compilation on platforms with dummy impl

v0.2.3 - 2019-08-06
-------------------

### Fixed

- xinput: Removed unneeded logging
- macos: `IS_Y_AXIS_REVERSED` is now correctly set to `true`
- macos: Fixed UUID calculation

v0.2.2 - 2019-04-06
-------------------

### Changed

- Windows: XInput is now dynamically loaded using rusty-xinput

### Fixed

- xinput: incorrect `is_connected()` after hotplugging
- wasm: Incorrect gamepad IDs in `Disconnected` event (@ryanisaacg)

v0.2.1 - 2019-02-25
-------------------

### Fixed

- Compilation error on macOS

v0.2.0 - 2019-02-21
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

- xinput: Incorrect gamepad ID when more than one gamepad is connected (
  @DTibbs).
