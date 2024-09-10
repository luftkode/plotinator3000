# Logviewer-rs

## Purpose

Inspect logs from the [Swiss Auto motor control](https://github.com/luftkode/mbed-motor-control) and generator. More log support can be added.



## Quick start

Check the [Justfile](Justfile) for recipes to get started, or invoke `just` to list available recipes.


Run as a native app

```
just run
```

Serve as WASM on a local web server

```
just serve
```

## Developer info

### The code

The logviewer is implemented with [eframe](https://github.com/emilk/egui/tree/master/crates/eframe), a framework for the [egui](https://github.com/emilk/egui) GUI library and for the plotting functionality, the [egui_plot](https://github.com/emilk/egui_plot) library is used.

All the boilerplate and workflows etc. is pulled from [this eframe template](https://github.com/emilk/eframe_template) which is kept up to date with `egui`/`eframe` and should be a good source for updating this project for recent `egui`/`eframe` versions.

### Infrastructure (CI, releases)

[cargo-dist](https://github.com/axodotdev/cargo-dist) handles the complexities of setting up build/releases for various platforms.
