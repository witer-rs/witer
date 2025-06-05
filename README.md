# <div align="center">`witer`</div> 

[<div align="center">![Crates.io Version](https://img.shields.io/crates/v/witer)](https://crates.io/crates/witer)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/R6R8PGIU6)
[![Discord](https://img.shields.io/discord/1215348294105169940?logo=discord&label=discord&color=5865F2)</div>](https://discord.gg/KEtfte9xWZ)

## An iterator-based Win32 window library built in Rust

> [!WARNING]
> I am currently working on a cross-platform version of this library: [`ventana`](https://github.com/GTLugo/ventana)

```rust
use witer::prelude::*;

fn main() {
  // Build
  let window = Window::builder()
    .with_title("My App")
    .with_size(LogicalSize::new(800.0, 500.0))
    .build()
    .unwrap();

  // Run
  for message in &window {
    if let Message::Key { .. } = message {
      println!("{message:?}");
    }
  }
}
```

## Goals

The main goal of `witer` is to have a simple, easy-to-use API. The target audience is developers looking to create
a window quickly, easily, and idiomatically. I aim to have feature-parity with `winit` eventually as a secondary goal.

Cross-platform support is highly unlikely, but pull requests are welcomed if anyone else wants to tackle it.
`egui` support is experimental and a point of active development. OpenGL context creation is similarly under active development.

> [!WARNING]
> This project is still hilariously incomplete and may contain major bugs. I am only one student, after all, so please report any and all issues or feature requests!

## Documentation

Documentation is a work-in-progress as the crate evolves. Don't expect much here yet, so if you have any
questions, either:

* Make a post in the Discussions tab on GitHub
* Send a message over Discord
* Dive into the codebase yourself

In the future, when the API settles down, I plan to make migration guides between releases. For now, I recommend
referencing the examples if there are any doubts.

## Cargo Features

* **`rwh_05` / `rwh_06`:** use the appropriate version of `raw-window-handle`. `rwh_06` is the default.

## Examples

You can find examples in [the examples folder](examples). You can also see the vulkano branch of
[foxy-rs/foxy](https://github.com/foxy-rs/foxy/tree/vulkano), which as of the time of writing is utilizing `witer`, but
is subject to change.

## Contact Me

You can get in contact through the discord linked at the top, or post in the Discussions tab on GitHub.

Alternatively, you can email me at <dev.gabriel.lugo@gmail.com>

## FAQ

**Q:** Why not `winit`?

**A:** While `winit` is the best choice for pretty much everyone, I found that multithreading the windows message pump
could lead to performance gains (unsubstantiated). Additionally, I was simply not satisfied with the way the `winit` API
looks and feels. If you are perfectly satisfied with what `winit` offers, then I recommend you stick with it.

**Q:** What happened to `ezwin`?

**A:** I wanted a name that was more inline with the ideas and goals behind the project. Additionally, I wanted a chance
to pull the project back to `v0.x.y` to reflect its volatile nature. This isn't currently possible with Crates.io (ignoring
yanking). Furthermore, the `v3.x.y` could be seen as confusing for those who didn't understand why they existed.
