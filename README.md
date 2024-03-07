# ezwin-rs

[![Static Badge](https://img.shields.io/badge/crates.io-ezwin?style=for-the-badge&color=E5AB37)](https://crates.io/crates/ezwin)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/R6R8PGIU6)

## `ezwin` is an easy-to-use Win32 windowing library

```rust
use ezwin::prelude::*;

fn main() {
  // Configure
  let settings = WindowSettings::default();

  // Build
  let window = Window::new().unwrap();

  // Run
  for message in window.as_ref() {
    if let Message::Window(..) = message {
      println!("{message:?}");
    }
  }
}
```

⚠️ This project is still very much a WIP; I am only one student, after all. ⚠️

## Goals

The main goal of `ezwin` is to have a simple, easy-to-use API. The target audience is game developers looking to create
a window quickly and easily. I aim to have feature-parity with `winit` eventually as a secondary goal.

Cross-platform support is unlikely, but pull requests are welcomed if anyone else wants to tackle it.

I would like to eventually transition from using `windows` to `windows-sys` to benefit from better compile times,
as the wrappers included in the former are redundant for this crate.

## What happened to the 3.0 version?

As this project is in flux, there was a temporary `3.0` version that was implemented which strayed from my vision of
the crate. I regret publishing that version, and have since yanked each of them off of crates.io. In the future, I intend
to be far more deliberate and considerate over what gets published rather than willy-nilly publishing the next big features.

## Cargo Features

* **`rwh_05` / `rwh_06`:** use the appropriate version of `raw-window-handle`. `rwh_06` is the default.

## Examples

You can find examples in [the examples folder](examples). You can also see the vulkano branch of
[foxy-rs/foxy](https://github.com/foxy-rs/foxy/tree/vulkano), which as of the time of writing is utilizing `ezwin`, but
is subject to change.
