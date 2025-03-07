# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

### Fixed

- Fix RUSTSEC-2025-0009 by updating ring

### Changed

- Set minimum support rust version (MSRV) and add CI check

### Dependencies

- `log`: 0.4.25 → 0.4.26 ([#204](https://github.com/luftkode/plotinator3000/pull/204))
- `serde`: 1.0.217 → 1.0.218 ([#204](https://github.com/luftkode/plotinator3000/pull/204))
- `tempfile`: 3.17.0 → 3.17.1 ([#204](https://github.com/luftkode/plotinator3000/pull/204))
- `chrono`: 0.4.39 → 0.4.40 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `getset`: 0.1.4 → 0.1.5 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `zip`: 2.2.2 → 2.2.3 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `thiserror`: 2.0.11 → 2.0.12 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `cargo-bins/cargo-binstall`: 1.11.0 → 1.11.2 ([#207](https://github.com/luftkode/plotinator3000/pull/207))
- `strum`: 0.26.3 -> 0.27.1
- `strum_macros`: 0.26.3 -> 0.27.1
- `hdf5`: 0.9.4 -> 0.10.1

## [1.10.0]

### Added

- Support for mbed PID log v6

## [1.9.1]

### Fixed

- Fix zoom reset would reset to the max plot bounds of every loaded data point even if it wasn't actively being shown in a plot area ([#200](https://github.com/luftkode/plotinator3000/issues/200))

## [1.9.0]

### Changed

- Account for bug in Swiss FW 4.2.0 that mixes up `vbat` and `is_fan_on` during logging
- Rework how zoom reset. Now resets on startup and when loading new files.

### Fixed

- fix mipmap without base not including first mipmap level

## [1.8.0]

### Added

- X-axis zoom: CTRL/⌘ + scroll.
- Y-axis zoom: CTRL/⌘ + ALT + scroll.

### Changed

- Remove all generic and unused mipmap implementations.
- No longer include first and last points unconditionally (no longer required for zoom reset), and avoid copying plots by always borrowing.
- Specify some optional dependencies as such
- Refactor and document just recipes to align with ADR on standard just recipes
- Remove Y-axis locking feature that is made obsolete with the new scroll modifiers

### Dependencies

- `cargo-bins/cargo-binstall`: 1.10.22 → 1.11.0 ([#194](https://github.com/luftkode/plotinator3000/pull/194))
- `crambl/dependabot-changelog-writer`: 1.0.0 → 1.0.2 ([#194](https://github.com/luftkode/plotinator3000/pull/194))
- `tempfile`: 3.16.0 → 3.17.0 ([#195](https://github.com/luftkode/plotinator3000/pull/195))

## [1.7.0]

### Added

- Add `profiling` feature with code to ease the ability for developers to profile `plotinator3000`.
- Add support for upcoming log version in Swiss auto FW v4.2

### Changed

- Organize test data files etc. in the new `test_util` crate with utilities for generating basic test boiler plate
- Set `CROSS_NO_WARNINGS=0` due to cross configuration conflict
- Combine min & max MipMaps instead of showing each as a separate line
- Performance: Reduce copying by utilizing the new `egui_plot` feature of borrowing `PlotPoints`.

### Dependencies

- Bump `openssl` from `0.10.68` -> `0.10.70` to fix RUSTSEC
- Bump all dependencies with `cargo update`
- Update egui from `0.30.0` to `0.31.0`

## [1.6.4]

### Added

- Added product icon for windows

## [1.6.3]

### Changed

- Click delta now needs the `shift` modifier to place a point for measuring distance between points
- Loaded log IDs are now guaranteed to be unique
- When a logs settings/metadata window is open, the plots from that log is highlighted
- Allow highlighting of plots from 2 logs by having the settings/metadata from one log open and hovering on another ones name

### Fixed

- Line width would not apply when displaying plots where down sampling was manually disabled

## [1.6.2]

### Changed

- MBED motor log `runtime` counter is displayed in hours instead of seconds

### Dependencies

- `getset`: 0.1.3 → 0.1.4 ([#166](https://github.com/luftkode/plotinator3000/pull/166))

## [1.6.1]

### Changed

- Click delta now shows days/hours/minutes/seconds/milliseconds depending on the magnitude of the distance between the points
- Click delta text offset increased slightly such that it is more likely to not intersect with the line
- Highlighting plot lines from a given log now also applies if other elements on the line with the given loaded log is hovered

## [1.6.0]

### Added

- View distance between specific points on the plot by clicking on them

### Fixed

- Font size now persists between sessions

## [1.5.1]

### Fixed

- Fix bad `u8` cast for `vbat`. Mbed v4 status log had changes to the log entries and still contained the bad cast that led to `vbat` losing float precision.

## [1.5.0]

### Added

- Support for MBED v4 logs

### Changed

- To reduce clotter, grids are now off by default

### Dependencies

- Update all dependencies to latest
- `cargo-bins/cargo-binstall`: 1.10.20 → 1.10.22 ([#161](https://github.com/luftkode/plotinator3000/pull/161))

## [1.4.0]

### Added

- Allow deleting individual loaded files without removing everything at once.
- Hovering the cursor over a loaded log will highlight the plots and plot labels that came from that log.

## [1.3.8]

### Fix

- When the update process requires admin (on windows) it now shows a prompt and relaunches as admin if the users chooses to continue.

### Changed

- Shows error context if errors occur during update process.
- No longer installs the updater binary (as the main binary handles the update process)

## [1.3.7]

### Fix

- Fixed `vbat` in mbed status log was accidentally cast to `u8`, losing a lot of precision in the process.

## [1.3.6]

### Fix

- ci: Cargo audit job changed to manually install with `cargo binstall` as `cargo-audit` is no longer installed on GitHub VMs by default
- fix: X11 support was accidentally removed when updating to egui `0.30`

### Dependencies

- `serde`: 1.0.215 → 1.0.217
- `chrono`: 0.4.38 → 0.4.39
- `semver`: 1.0.23 → 1.0.24
- `env_logger`: 0.11.5 → 0.11.6
- `zip`: 2.2.1 → 2.2.2
- `rfd`: 0.15.1 → 0.15.2 ([#147](https://github.com/luftkode/plotinator3000/pull/147))
- `tempfile`: 3.14.0 → 3.15.0 ([#147](https://github.com/luftkode/plotinator3000/pull/147))
- `reqwest`: 0.12.9 → 0.12.12 ([#147](https://github.com/luftkode/plotinator3000/pull/147))
- `tokio`: 1.42.0 → 1.43.0 ([#149](https://github.com/luftkode/plotinator3000/pull/149))
- `wasm-bindgen-futures`: 0.4.47 → 0.4.50 ([#149](https://github.com/luftkode/plotinator3000/pull/149))
- `web-sys`: 0.3.74 → 0.3.77 ([#149](https://github.com/luftkode/plotinator3000/pull/149))
- `thiserror`: 2.0.4 → 2.0.11 ([#149](https://github.com/luftkode/plotinator3000/pull/149))

## [1.3.5]

### Changed

- Update egui from `0.29.0` to `0.30.0`
- Port fixes from `eframe_template`

## [1.3.4]

### Changed

- Only set font styles when it changes instead of at every frame (leftover tech-debt from starting to learn egui)
- Make the loaded files window scrollable - resolves https://github.com/luftkode/plotinator3000/issues/118

## [1.3.3]

### Changed

- Update dependencies

## [1.3.2]

### Changed

- Bump Rust compiler from `1.81.0` to `1.82.0`
- Bump dependencies

## [1.3.1]

### Fix

- Selfupdater failing to determine install receipt prevents it from doing an upgrade

## [1.3.0]

### Added

- Show/hide all button for loaded files window

### Changed

- Bump `thiserror` to 2.0
- Bump dependencies

## [1.2.2]

### Fixed

- Self-updater has been fixed and re-enabled

## [1.2.1]

### Changed

- Update Mbed log v4 with the new config changes.

## [1.2.0]

### Changed

- Re-enable installation of an updater executable
- MBED logs now normalize `servo duty cycle` to 0-100%. The full range of the servo duty cycle is [0; 0.1].

### Internal

- Bump cargo-dist from 0.23.0 -> 0.25.1
- Bump axoupdater from 0.7.3 -> 0.8.1

## [1.1.2]

### Changed

- Guard self-updater behind feature flag as it is currently broken.

## [1.1.1]

### Changed

- When auto downsampling is enabled and the mipmap level for a given plot is determined to be 1, we use all data points instead of downsampling to level 1. When we are plotting a downsampled min and max, level 1 is just as many data points as the original non-downsampled plot, so this is a strictly better solution.

## [1.1.0]

### Added

- Support for Mbed log v3
- Preliminary support for Mbed log v3

## [1.0.2]

### Fix

- Updater was looking for an install receipt which plotinator3000 no longer uses. The updater now proceeds without needing an install receipt.

## [1.0.1]

### Changed

- Added a notification whenever a log is loaded, showing the total data points of loaded files.

### Internal

- Cleanup unused library code
- Prepare support for a new version of the mbed config present in mbed log headers.

## [1.0.0]

### Changed

- `logviewer-rs` is now renamed to `Plotinator3000`, signifying that it is not really a logviewer and more of a plotting app that will plot any supported format, and do it very fast.

## [0.28.0]

### Added

- Auto updater that queries for newer versions and opens an installer window if a new update is available

## [0.27.0]

### Added

- Support for the `NavSys.sps` format.

## [0.26.0]

### Fix

- Plot alignment

### Changed

- Make some UI elements smaller
- Allow main window (viewport) to be shrink much more than before
- Plot setting UI elements wrap instead of stay fixed when window shrinks

## [0.25.0]

### Added

- File dialog for native and web, which also allows mobile users to load logs.

### Changed

- Various UI tweaks
- Clean up some outdated error messages.

### Internals

- Decouple file parsing from file loading method.

## [0.24.1]

### Fix

- Web version of `plotinator3000` was broken due an integer overflow. When determining down sample level, a cast from 64-bit float to pointer size caused integer overflow on wasm due to wasm having a 32-bit pointer size.

## [0.24.0]

### Added

- Initial support for `HDF` files, starting with bifrost (TX) loop current. The feature is currently guarded behind a feature flag, enabling it is tracked at: https://github.com/luftkode/plotinator3000/issues/84.

### Changed

- Various UI tweaks

### Internal

- Upgraded `cargo-dist` `0.22.1` -> `0.23.0`

## [0.23.0]

### Added

- A warning notification is now shown if a log was parsed from contents where more than 128 bytes of the content was not recognized as log content (and therefor skipped)
- When viewing log info, the first line shows parse info, how many bytes were parsed out of the total length of the file.

### Changed

- Plot labels are now sorted alphabetically
- Remove unused `T_SHUTDOWN` config value that was not supposed to be in mbed log v2.
- Avoid downsampling all the way to 2 samples by setting a minimum downsample threshold (set to 512 samples)
- Avoid storing redundant copies of source plot data when creating multiple mipmaps from the same source.

### Internals

- Refactor to simplify mipmap configuration

## [0.22.0]

### Changed

- Plots retain the color they originally were assigned, no matter if logs are toggled to be invisible or a plot filter is hiding some plots.
- Min/max downsampled plots now have the same color

### Internals

- Refactor to reduce a bunch of duplication and tech debt

## [0.21.0]

### Changed

- Much faster way of determining which plot points that fit within plot boundings.
- Avoid double work when auto downsampling is enabled, previously the fitting downsampling level was first found before handing off that level to a filtering function, which would find partition bounds that were already known from finding the fitting downsampling level.

## [0.20.0]

### Added

- Display log metadata when clicking on a loaded log.

### Fix

- `Mbed status Log v2` mistakenly interpreted as `Mbed status Log v1` when loaded via a path (native `plotinator3000`)

## [0.19.0]

### Added

- Support for Mbed Log version 2

## [0.18.4]

### Changed

- Remove playback feature

## [0.18.3]

### Fix

- Integer overflow when searching for the appropriate downsampled MipMap level.

## [0.18.2]

### Changed

- Much faster implementation for finding the appropriate MipMap level for the current zoom level.

### Fix

- Min/Max MipMap algorithm flipped min and max.

## [0.18.1]

### Fix

- Fix accidentally using plot **name** instead of plot **label**, causing name conflicts when plotting multiple plots with the same name
- Bad naming in a PID controller implementation caused misunderstanding, what we thought was the PID error was actually the PID output.

## [0.18.0]

### Added

- Min/Max MipMap downsampling which makes plotting much larger datasets feasible, and facilitates outlier detection.

### Internals

- Major clearing of tech debts related to plot settings and the ui for plot settings.

## [0.17.0]

### Added

- Allow toggling whether or not plots from each loaded log are shown.

### Internals

- Update dependencies

## [0.16.0]

### Added

- Hovering on a plot now also shows the plot name.
- Allow Filtering the shown plots by legend/label name.

### Changed

- (Native only) Recursively parsing drag-n-dropped directories also parses zip archives
- Reduce UI by allowing toggling the list of logs.
- Reduce UI cluttering by removing the "Time" label on the X-axis.
- Reduce verbosity of the name of logs
- Better visuals for viewing and changing settings of loaded logs

### Internals

- Refactors

## [0.15.0]

### Added

- (Not available on web version) Recursively parse drag-n-dropped directories for supported logs
- (Not available on web version) Recursively parse drag-n-dropped zip archives for supported logs
- Show tooltip when hovering above a clickable log name

### Internals

- Refactors

## [0.14.0]

### Add

- Ability for logs to add labels (rough initial mvp, needs more work)
- `show grid` button for showing/hiding grid lines on plots

## [0.13.0]

### Internals

- Better divide interfaces in the `log_if` crate and use a prelude to make it easy to import all relevant traits via glob imports.
- Change `GitMetadata` trait to return `Option<String>` to accommodate logs that don't contain git metadata.

### Changes

- Mbed motor control PID log's `RPM Error Count` and `First Valid RPM Count` are moved to the plot ranging from 1-100.

## [0.12.0]

### Changes

- Allow Setting the date/offset of plots with the `Enter`-key and closing the settings window by pressing `Escape`
- Upgrade `egui` to v0.29
- Y-axis lock is now compatible with all zoom and scroll actions

### Internals

- Migrate to workspace project structure
- Decouple Log implementation from the plotting interface
