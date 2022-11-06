Change Log
==========

v0.5.0 - 2022-11-06
--------------------

### Changed

- Windows now defaults to using Windows Gaming Input instead of xinput.

  If you need to use xInput you can disable the `wgi` feature (It's enabled by default) and enable the `xinput` feature.
  ``` toml
  gilrs-core = {version = "0.5.0", default-features = false, features = ["wgi"]}
  ```
- Apps on Windows will now require a focused window to receive inputs by default.

  This is a limitation of Windows Gaming Input. It requires an in focus Window be associated with the process to receive events. You can still switch back to using xInput by turning off default features and enabling the `xinput` feature.

### Fixed

- `Gamepad::axes()` on macos now also returns "hat" axes. This should fix dpad on single Switch Joy-Con.

v0.4.1 - 2022-05-29
-------------------

### Changed

- Updated io-kit-sys to 0.2 and core-foundation to 0.9 (@jtakakura).
- Reduced numer of enabled features for nix crate (@rtzoeller).

v0.4.0 - 2022-05-22
-------------------

### Changed

- wasm: web-sys/wasm-bindgen is now used by default, dependency on stdweb  and `wasm-bindgen` feature are removed.
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

- xinput: Incorrect gamepad ID when more than one gamepad is connected (@DTibbs).
