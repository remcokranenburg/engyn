# Engyn

Engyn will be an experimental VR graphics engine, designed to automatically tune its quality to
reach a target frame rate. To this end, every graphical feature will be highly configurable and
detailed performance breakdowns will be generated that can be used to decide how to tune the
algorithms.

Currently, you can walk around in a simple static environment without any lighting.

## How do I run this?

This program is written in Rust. You will need the latest nightly release, which can be obtained
with [Rustup](https://rustup.rs/). If you're on Windows, choose the MSVC variant when you're
installing Rust from Rustup.

Then, in the root of the project run the following:

```
cargo build
cargo run
```

When you're on a Wayland-enabled Linux, you should run it with WAYLAND_DISPLAY="" to work around a
bug in glium, so put that before `cargo run` or in your .bash_profile. The VR support (when merged
in) will only work on Windows for the time being. Linux support depends on Valve's impending Linux
release of SteamVR.

## License

The project as a whole is licensed under the [GPL, version 3](GPL-3.0.md). Third-party resources are
licensed under their own terms, as listed in [LICENSE.md](LICENSE.md).
