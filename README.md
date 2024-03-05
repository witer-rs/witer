# ezwin-rs

[![Static Badge](https://img.shields.io/badge/crates.io-ezwin?style=for-the-badge&color=E5AB37)](https://crates.io/crates/ezwin)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/R6R8PGIU6)

```rust
use ezwin::prelude::*;

struct App {}

// Implement
impl WindowProcedure<()> for App {
  fn on_create(_: &Arc<Window>, _: ()) -> Option<Self> {
    Some(Self {})
  }

  fn on_message(&mut self, _: &Arc<Window>, _: Message) {}
}

fn main() {
  // Configure
  let settings = WindowSettings::<App, _>::new(())
    .with_size((1280, 720))
    .with_title("Example");

  // Build
  let window = Window::new(settings).unwrap();

  // Run
  while window.pump() {}
}
```

## `ezwin` is an easy-to-use Win32 windowing library

⚠️ This project is still very much a WIP; I am only one student, after all. ⚠️

## Goals

The main goal of `ezwin` is to have a simple, easy-to-use API. The target audience is game developers looking to create
a window quickly and easily. I aim to have feature-parity with `winit` eventually as a secondary goal.

Cross-platform support is unlikely, but pull requests are welcomed if anyone else wants to tackle it.

I would like to eventually transition from using `windows` to `windows-sys` to benefit from better compile times,
as the wrappers included in the former are redundant for this crate.

## Examples

You can find examples in [the examples folder](examples). You can also see the vulkano branch of
[foxy-rs/foxy](https://github.com/foxy-rs/foxy/tree/vulkano), which as of the time of writing is utilizing `ezwin`, but
is subject to change.

## Why the rework?

I have to agree that the original API for `ezwin` was much nicer to work with, but it had a number of issues related to
the multithreaded nature of things. The main issue which was the most difficult to solve, and I never could, was that of
waiting for the previous message to complete before processing the next message without creating deadlocks. This is
important to prevent issues related to resizing windows and to prevent excessive delays with input. I do plan on
investigating further in the future a way to reintroduce an interator-based API, but for now, the rework should get
things working properly.

## Cargo Features

* **`rwh_05` / `rwh_06`:** use the appropriate version of `raw-window-handle`. `rwh_06` is the default.
