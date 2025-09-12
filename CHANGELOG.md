# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

## [1.32.0]

- Add boilerplate for supporting CSV file formats
- Add support for CSV file with Njord INS PPP correction data
- Add support for GrafNav PPP file (csv-like)

## [1.31.0]

- Support new Njord INS format with consolidated GPS timestamps and system time datasets

## [1.30.1]

- Fix `NaN` values were added for navsys sps kitchen sink GP1

## [1.30.0]

### Added

- Parse `frame-gps` MQTT packets

### Fixed

- Avoid adding `NaN` values from `frame-gps` to datasets as they cause problems for the plotter
- Detect `NaN` strings in navsys kitchen sink and ignore them when building datasets

### Dependencies

- `chrono`: 0.4.41 → 0.4.42 ([#335](https://github.com/luftkode/plotinator3000/pull/335))

## [1.29.1]

- log warning instead of adding invalid gp time offset when `frame-gps`'s `gps_time` doesn't have a valid timestamp

## [1.29.0]

- Update dependencies
- Optimize loading Njord-Ins datasets by parallelizing the HDF5 reading and preprocessing
- Paralellize downsampling (mipmap generation) if the loaded dataset is very large
- Various other minor optimizations

## [1.28.0]

- Add support for `frame-gps` HDF5 files

## [1.27.3]

### Changed

- Disable periodically saving app state if large datasets are loaded to prevent periodic lag spikes
- Format very large numbers in a more human readable way

### Dependencies

- `zip`: 4.4.0 → 4.5.0 ([#323](https://github.com/luftkode/plotinator3000/pull/323))
- `memmap2`: 0.9.7 → 0.9.8 ([#323](https://github.com/luftkode/plotinator3000/pull/323))

## [1.27.2]

- Fix crash when parsing date from malformed `TSC.h5` GPS data (where all date data is 0)

## [1.27.1]

- Avoid loading the whole HM dataset of `TSC.h5` when only the shape is used

## [1.27.0]

- Add plot for TSC timestamp delta
- Add timestamp statistics to TSC metadata
- Add show/hide all button for `[bool]` data series

## [1.26.0]

- Always try to fetch WASM bundle, fallback to cached if fetch fails
- Add support for `TSC.h5` GPS data

## [1.25.0]

- Support parsing HDF5 files from zip archives
- Recursively parse zip archives up to a max depth of 3

## [1.24.0]

- Filter invalid values from the `frame-altimeter` into a "invalid values" dataset, just like with `navsys`
- Group filter plot labels after log type
- Group loaded logs after type
- Sort loaded logs alphabetically and after start time
- Show loaded logs start datetime as `YYYY-MM-DD HH:MM:SS`

## [1.23.0]

- Apply highlighting to polygons in scatter plots in the same way as it is applied in line plots
- Improve filter window UI
- Highlights plots that are effected by a filter label when hovering the given label

## [1.22.1]

- Update dependencies
- Close download window on `Esc`

## [1.22.0]

### Added

- **Experimental** download latest data files from TS

### Fixed

- `frame-inclinometer` would show `pitch (old)` to indicate the old incorrect calculation, but that actually implies to `roll` and not `pitch`
- Sps kitchen sink did not recognize inclinometer calibration values as valid entries

## [1.21.1]

### Fixed

- Recognize navsys entries with ID 3 for `navsys kitchen sink`, and fallback to "x" for any IDs above 3
- Allow `frame-altimeters` HDF5 files to only have data from one altimeter

### Dependencies

- `egui`: 0.32.0 → 0.32.1 ([#304](https://github.com/luftkode/plotinator3000/pull/304))
- `eframe`: 0.32.0 → 0.32.1 ([#304](https://github.com/luftkode/plotinator3000/pull/304))
- `thiserror`: 2.0.12 → 2.0.15 ([#304](https://github.com/luftkode/plotinator3000/pull/304))
- `anyhow`: 1.0.98 → 1.0.99 ([#304](https://github.com/luftkode/plotinator3000/pull/304))
- `reqwest`: 0.12.22 → 0.12.23 ([#304](https://github.com/luftkode/plotinator3000/pull/304))

## [1.21.0]

### Added

- Support for `Frame Inclinometers` HDF5 files
- Support for `Frame Magnetometer` HDF5 files
- Support for standalone "kitchen sink" SPS files with any valid Navsys.sps entries

### Changed

- The log metadata in the loaded files window is scrollable, to support cases where log files have a lot of metadata

### dependencies

- run `cargo update`

## [1.20.2]

### Changed

- Move back to original `cargo-dist` project instead of the `uv` fork now that the original is maintained again

### Dependencies

- `tokio`: 1.47.0 → 1.47.1 ([#290](https://github.com/luftkode/plotinator3000/pull/290))
- `serde_json`: 1.0.141 → 1.0.142 ([#290](https://github.com/luftkode/plotinator3000/pull/290))
- `toml`: 0.9.2 → 0.9.4 ([#290](https://github.com/luftkode/plotinator3000/pull/290))

## [1.20.1]

### Changed

- Update known MQTT topics

## [1.20.0]

### Added

- Allow configuring the radius of scatter plot point indicators via the `width` setting.

### Changed

- Tweaks to improve the configurability of the new draw modes.

## [1.19.0]

### Added

- Select between 3 modes for drawing a point series: Line plot, scatter plot, and line plot with point emphasis.
- When hovering over plot area visibility buttons, the area the button couples to is highlighted.

### Dependencies

- Run `cargo update`
- `rfd`: 0.15.3 → 0.15.4 ([#283](https://github.com/luftkode/plotinator3000/pull/283))
- `memmap2`: 0.9.5 → 0.9.7 ([#283](https://github.com/luftkode/plotinator3000/pull/283))

## [1.18.1]

### Changed

- Parse custom plot data files from zip archives as well
- Update `toml` to latest
- Update `zip` to latest

### Fixed

- Erroneous attempts at opening dropped directories as files

## [1.18.0]

### Added

- Ability to export Plot data as special `p3k` plotinator3000 files (includes data collected with MQTT)
- Ability to export the whole plot ui state (including plot data) as special `p3k` plotinator3000 files.
- Supported files filter for file dialogs

## [1.17.0]

### Fixed

- Click delta text now has a contrasted background color, making it easy to read

### Changed

- Update egui to `0.32.0` which brings a bunch of features and major improvement
- Dragging axes now zooms according to which way its dragged
- Less wasted space between plot areas
- Better text rendering, etc. etc.

## [1.16.0]

### Added

- MQTT Connection now clearly indicates status and attempts to reconnect

### Changed

- MQTT Connect window saves broker addr and allows easily re-configuring the connection

### Dependencies

- `tokio`: 1.45.1 → 1.46.1 ([#274](https://github.com/luftkode/plotinator3000/pull/274))
- `reqwest`: 0.12.20 → 0.12.22 ([#274](https://github.com/luftkode/plotinator3000/pull/274))
- `toml` to latest `0.9.1`

## [1.15.0]

### Added

- Initial support for the new `njord-ins` HDF5 file
- Support for the new `frame-altimeter` HDF5 file
- Refactor HDF5 to have much less boilerplate

### Dependencies

- `reqwest`: 0.12.19 → 0.12.20 ([#267](https://github.com/luftkode/plotinator3000/pull/267))
- `getset`: 0.1.5 → 0.1.6 ([#271](https://github.com/luftkode/plotinator3000/pull/271))
- `zip`: 4.0.0 → 4.2.0 ([#271](https://github.com/luftkode/plotinator3000/pull/271))
- `profiling`: 1.0.16 → 1.0.17 ([#271](https://github.com/luftkode/plotinator3000/pull/271))
- `mimalloc`: 0.1.46 → 0.1.47 ([#271](https://github.com/luftkode/plotinator3000/pull/271))
- `cargo-bins/cargo-binstall`: 1.12.7 → 1.14.1 ([#270](https://github.com/luftkode/plotinator3000/pull/270))
- `crambl/dependabot-changelog-writer`: 1.1.4 → 1.3.0 ([#272](https://github.com/luftkode/plotinator3000/pull/272))

## [1.14.3]

### Changed

- Bifrost current now also shows a "combined" line plot
- Unify crate naming scheme
- Reduce boilerplate of introducing new logs/hdf5 files with some simple macros
- Move supported format definitions to separate crate
- Move updater to separate crate

### Dependencies

- `cargo-bins/cargo-binstall`: 1.12.5 → 1.12.7 ([#260](https://github.com/luftkode/plotinator3000/pull/260))
- `reqwest`: 0.12.18 → 0.12.19 ([#261](https://github.com/luftkode/plotinator3000/pull/261))

## [1.14.2]

### Fixed

- Fix [#250](https://github.com/luftkode/plotinator3000/issues/250) crash when the calculated distance between x-axis grid marks is less than machine epsilon
- Fix crash on timestamp out of range when X-axis stretches >100 years

### Dependencies

- `reqwest`: 0.12.15 → 0.12.18 ([#254](https://github.com/luftkode/plotinator3000/pull/254))

## [1.14.1]

### Fixed

- Update check at startup would prevent app creation if no connection was available, now skips updater on request failure.

## [1.14.0]

### Added

- Support for Wasp200/Njord altimeter hdf5 and .sps files

### Changed

- Don't include empty hdf5 attributes under loaded file descriptions

### Dependencies

- `tokio`: 1.45.0 → 1.45.1 ([#248](https://github.com/luftkode/plotinator3000/pull/248))
- `zip`: 3.0.0 → 4.0.0 ([#248](https://github.com/luftkode/plotinator3000/pull/248))

## [1.13.3]

### Changed

- Update pilot display related MQTT topics

## [1.13.2]

### Changed

- Vastly improved X-axis time labelling

## [1.13.1]

### Fixed

- Fix long standing issue where zooming to a high enough degree could cause plot lines to not display all the points that should be visible

## [1.13.0]

### Added

- Discover broker `$SYS`-topics
- Show reachable broker's version if available
- Better UX for the MQTT connection window
- Add additional distribution targets: `ARM64 Linux` & `x64 MUSL Linux`.

### Changed

- Switch to `mimalloc` as the global allocator for significant performance improvements (~20%)

### Dependencies

- Run `cargo update`
- `zip`: 2.6.1 → 3.0.0 ([#243](https://github.com/luftkode/plotinator3000/pull/243))
- `cargo-bins/cargo-binstall`: 1.12.3 → 1.12.4 ([#238](https://github.com/luftkode/plotinator3000/pull/238))

### Internal

- Better encapsulation of MQTT features in GUI code
- Added GUI tests, including snapshot tests

## [1.12.0]

### Changed

- Move from the discontinued original `cargo-dist` to the [fork maintained by astral](https://github.com/astral-sh/cargo-dist)
- Use the forked `cargo-dist` with the MSVC CRT linker configuration fix in the release workflow (astral [PR here](https://github.com/astral-sh/cargo-dist/pull/36))
- Avoid statically linking MSVC CRT to allow statically linking HDF5 on windows.
- Disallow bypassing updates in CI, to force tests to run update scenarios

### Dependencies

- `anyhow`: 1.0.97 → 1.0.98 ([#228](https://github.com/luftkode/plotinator3000/pull/228))

## [1.11.1]

### Fixed

- Fix crash when parsing a Navsyssps-file where some sensor IDs are not present.
- Update crossbeam to fix [RUSTSEC-2025-0024](https://rustsec.org/advisories/RUSTSEC-2025-0024)

### Changed

- Update rust version and edition to latest

### Dependencies

- `log`: 0.4.26 → 0.4.27 ([#218](https://github.com/luftkode/plotinator3000/pull/218))
- `tempfile`: 3.19.0 → 3.19.1 ([#218](https://github.com/luftkode/plotinator3000/pull/218))
- `reqwest`: 0.12.14 → 0.12.15 ([#218](https://github.com/luftkode/plotinator3000/pull/218))
- `cargo-bins/cargo-binstall`: 1.12.1 → 1.12.2 ([#217](https://github.com/luftkode/plotinator3000/pull/217))
- `crambl/dependabot-changelog-writer`: 1.0.2 → 1.0.3 ([#219](https://github.com/luftkode/plotinator3000/pull/219))
- `openssl ` 0.10.70 -> 0.10.72
- `openssl-sys` 0.9.105 -> 0.9.107
- `tokio`: 1.44.1 → 1.44.2 ([#221](https://github.com/luftkode/plotinator3000/pull/221))
- `env_logger`: 0.11.7 → 0.11.8 ([#221](https://github.com/luftkode/plotinator3000/pull/221))
- `zip`: 2.3.0 → 2.6.1 ([#221](https://github.com/luftkode/plotinator3000/pull/221))

## [1.11.0]

### Added

- Plotinator3000 can now connect a specified MQTT broker and continuously plot data from selected topics

## [1.10.1]

### Fixed

- Fix RUSTSEC-2025-0009 by updating ring
- Fix issue where under certain conditions, plots would disappear from the legend and plot colors would shift around if a plot was far outside the view

### Changed

- Set minimum support rust version (MSRV) and add CI check

### Dependencies

- `log`: 0.4.25 → 0.4.26 ([#204](https://github.com/luftkode/plotinator3000/pull/204))
- `chrono`: 0.4.39 → 0.4.40 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `getset`: 0.1.4 → 0.1.5 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `thiserror`: 2.0.11 → 2.0.12 ([#206](https://github.com/luftkode/plotinator3000/pull/206))
- `strum`: 0.26.3 -> 0.27.1
- `strum_macros`: 0.26.3 -> 0.27.1
- `hdf5`: 0.9.4 -> 0.10.1
- `serde`: 1.0.217 → 1.0.219 ([#211](https://github.com/luftkode/plotinator3000/pull/211))
- `semver`: 1.0.25 → 1.0.26 ([#211](https://github.com/luftkode/plotinator3000/pull/211))
- `egui`: 0.31.0 → 0.31.1 ([#211](https://github.com/luftkode/plotinator3000/pull/211))
- `eframe`: 0.31.0 → 0.31.1 ([#211](https://github.com/luftkode/plotinator3000/pull/211))
- `rfd`: 0.15.2 → 0.15.3 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `tokio`: 1.43.0 → 1.44.1 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `env_logger`: 0.11.6 → 0.11.7 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `zip`: 2.2.2 → 2.3.0 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `tempfile`: 3.17.0 → 3.19.0 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `reqwest`: 0.12.12 → 0.12.14 ([#212](https://github.com/luftkode/plotinator3000/pull/212))
- `cargo-bins/cargo-binstall`: 1.11.0 → 1.12.1 ([#213](https://github.com/luftkode/plotinator3000/pull/213))

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

- Organize test data files etc. in the new `plotinator-test-util` crate with utilities for generating basic test boiler plate
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

- Better divide interfaces in the `plotinator-log-if` crate and use a prelude to make it easy to import all relevant traits via glob imports.
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
