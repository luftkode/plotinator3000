<div align="center">
  <a href="https://github.com/luftkode/plotinator3000/releases" title="Latest Stable GitHub Release">
      <img src="https://img.shields.io/github/release/luftkode/plotinator3000/all.svg?style=flat&logo=github&logoColor=white&colorB=blue&label=Latest Release" alt="GitHub release"></a>
  <a href="https://github.com/luftkode/plotinator3000/actions/workflows/CI.yml"><img src="https://github.com/luftkode/plotinator3000/actions/workflows/CI.yml/badge.svg" alt="CodeFactor" /></a>
</div>
<div align="center">
    <img src="https://img.shields.io/badge/-Windows-6E46A2.svg?style=flat&logo=windows-11&logoColor=white" alt="Windows" title="Supported Platform: Windows">&thinsp;
    <img src="https://img.shields.io/badge/-Linux-9C2A91.svg?style=flat&logo=linux&logoColor=white" alt="Linux" title="Supported Platform: Linux">&thinsp;
    <img src="https://img.shields.io/badge/-macOS-red.svg?style=flat&logo=apple&logoColor=white" alt="macOS" title="Supported Platform: macOS">
</div>


# SkyTEM Plotinator3000

## Purpose

Inspect logs from the [Swiss Auto motor control](https://github.com/luftkode/mbed-motor-control) and generator. More log support will be added as needed.

## Why is this repository public?

For inspiration/educational purposes. Anyone developing `egui`/`eframe` apps may or may not find any of the solutions in this repository useful for their own project(s).

## Installation

See the [latest release](https://github.com/luftkode/plotinator3000/releases/latest) and choose the installation method you prefer.

Installing with the shell script (unix) or powershell script (windows) will also install an updater which can be used to fetch the latest version.

## Use in browser: https://plotinator3000.web.app/

A more stripped down version is also available as a web app at https://plotinator3000.web.app/ it lacks certain features like drag-n-dropping zip-files and directories, and will likely never have HDF5 support.

## Quick start

Check the [Justfile](Justfile) for recipes to get started, or invoke `just` to list available recipes.

Check if code compiles on native and wasm targets without actually compiling

```shell
just check-all # alias `ca` for speed!
```

Run as a native app

```shell
just run
```

Serve as WASM on a local web server

```shell
just serve
```

## Developer info

### The code

The plotinator3000 is implemented with [eframe](https://github.com/emilk/egui/tree/master/crates/eframe), a framework for the [egui](https://github.com/emilk/egui) GUI library and for the plotting functionality, the [egui_plot](https://github.com/emilk/egui_plot) library is used.

All the boilerplate and workflows etc. is pulled from [this eframe template](https://github.com/emilk/eframe_template) which is kept up to date with `egui`/`eframe` and should be a good source for updating this project for recent `egui`/`eframe` versions.

### Infrastructure (CI, releases)

[cargo-dist](https://github.com/axodotdev/cargo-dist) handles the complexities of setting up build/releases for various platforms.

Read their documentation!!

Generating the first instance of CI for the release workflow is done via

```shell
cargo dist init
```
... And then following the instructions/prompts.

A (very complicated) [release.yml](.github/workflows/release.yml) is generated and metadata is added to [Cargo.toml](Cargo.toml), if distributing for windows, a [main.wxs](wix/main.wxs) is also generated. To update these with changes to the project, simply rerun `cargo dist init`, don't edit the workflow manually, there's a [section on CI customization](https://opensource.axo.dev/cargo-dist/book/ci/customizing.html) in the `cargo dist` docs.

When developing/trouble shooting the release pipeline adding `pr-run-mode = "upload"` like this

```toml
[workspace.metadata.dist]
pr-run-mode = "upload"
```

Will run the release pipeline on pull request, then you can open a PR and develop/fix/test the release pipeline.
