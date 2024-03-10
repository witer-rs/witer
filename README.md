# ezwin: A Minimal native Win32 window library

## NOTICE: Further development will be done as the [`witer`](https://crates.io/crates/witer) crate

[![Crates.io Version](https://img.shields.io/crates/v/ezwin)](https://crates.io/crates/ezwin)
[![Discord](https://img.shields.io/discord/1215348294105169940?logo=discord&label=discord&color=5865F2)](https://discord.gg/KEtfte9xWZ)

```rust
use ezwin::prelude::*;

fn main() {
  // Configure
  let settings = WindowSettings::default();

  // Build
  let window = Window::new(settings).unwrap();

  // Run
  for message in &window {
    if let Message::Window(..) = message {
      println!("{message:?}");
    }
  }
}
```

## Goals

The main goal of `ezwin` is to have a simple, easy-to-use API. The target audience is developers looking to create
a window quickly, easily, and idiomatically. I aim to have feature-parity with `winit` eventually as a secondary goal.

Cross-platform support is highly unlikely, but pull requests are welcomed if anyone else wants to tackle it.

I would like to eventually transition from using `windows` to `windows-sys` to benefit from better compile times,
as the wrappers included in the former are redundant for this crate.

## Documentation

Documentation is a work-in-progress as the crate evolves. Don't expect much here yet, so if you have any
questions, either:

* Make a post in the Discussions tab on GitHub
* Send a message over Discord
* Dive into the codebase yourself

## Cargo Features

* **`rwh_05` / `rwh_06`:** use the appropriate version of `raw-window-handle`. `rwh_06` is the default.

## Examples

You can find examples in [the examples folder](examples). You can also see the vulkano branch of
[foxy-rs/foxy](https://github.com/foxy-rs/foxy/tree/vulkano), which as of the time of writing is utilizing `ezwin`, but
is subject to change.

## Contact Me

You can get in contact through the discord linked at the top, or post in the Discussions tab on GitHub.

## FAQ

**Q:** Why not `winit`?

**A:** While `winit` is the best choice for pretty much everyone, I found that multithreading the windows message pump
could lead to performance gains (unsubstantiated). Additionally, I was simply not satisfied with the way the `winit` API
looks and feels. If you are perfectly satisfied with what `winit` offers, then I recommend you stick with it.

**Q:** What happened to the 3.0 version?

**A:** As this project is in flux, there was a temporary `3.0` version that was implemented which strayed from my vision of
the crate. I regret publishing that version, and have since yanked each of them off of crates.io. In the future, I intend
to be far more deliberate and considerate over what gets published rather than willy-nilly publishing the next big features.

## ⚠️ Warning ⚠️

This project is still hilariously incomplete. I am only one student, after all.

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/R6R8PGIU6)
