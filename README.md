# ezwin-rs

[![Static Badge](https://img.shields.io/badge/crates.io-ezwin?style=for-the-badge&color=E5AB37)](https://crates.io/crates/ezwin)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/R6R8PGIU6)

```rust
use ezwin::prelude::*;

fn main() -> WindowResult<()> {
  let window = Window::new(
    WindowSettings::default()
      .with_close_on_x(false)
      .with_flow(Flow::Wait)
      .with_title("Easy Window")
      .with_size((800, 600)),
  )?;

  for msg in &window {
    if let Message::CloseRequested = msg {
      window.close();
    }

    println!("{:?}", msg)
  }

  Ok(())
}
```

## `ezwin` is an easy-to-use Win32 windowing library

⚠️ This project is still very much a WIP; I am only one student, after all. ⚠️

The main goal of `ezwin` is to have a simple, easy-to-use API. The target audience is game developers looking to create
a window quickly and easily. I aim to have feature-parity with `winit` eventually as a secondary goal. Cross-platform 
support is unlikely, but pull requests are welcomed if anyone else wants to tackle it.

There are **2** primary threads in `ezwin`:

* **main:** where all the main user code is executed.
* **window:** where the window is created and the message pump lives.

This layout was chosen to allow for the window messages not to block the application.

## Cargo Features

* **`rwh_05` / `rwh_06`:** use the appropriate version of `raw-window-handle`. `rwh_06` is the default.