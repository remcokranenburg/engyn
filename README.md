# Engyn

Engyn will be an experimental VR graphics engine, designed to automatically tune its quality to
reach a target frame rate. To this end, every graphical feature will be highly configurable and
detailed performance breakdowns will be generated that can be used to decide how to tune the
algorithms.

Currently, you can walk around in a simple static environment with basic dynamic lighting. In
VR-mode, your controllers are visible. In regular mode, the text "Hello, world!" is drawn on top in
the middle of the screen.

## How do I run this?

1. This program is written in Rust. You will need the latest nightly release, which can be obtained
    with [Rustup](https://rustup.rs/). If you're on Windows and you want VR support, choose the MSVC
    variant when you're installing Rust from Rustup.

2. Then, in the root of the project run the following:

    ```
    cargo build
    ```

3. Optional: if you want VR support, you'll need to [click here][dll] to download openvr_api.dll
   from Valve's repository and put it next to the `engyn.exe` binary in the `target/debug/`
   directory.

4. Now you can run the program, like so:

    ```
    cargo run
    ```

## Troubleshooting

When you're on a Wayland-enabled Linux, you should run it with WAYLAND_DISPLAY="" to work around a
bug in glium, so put that before `cargo run` or in your .bash_profile.

The VR support will only work on Windows for the time being. Linux support depends on Valve's
impending Linux release of SteamVR.

## License

The project as a whole is licensed under the [GPL, version 3 or later](GPL-3.0.md). Third-party
resources are licensed under their own terms, as listed in [LICENSE.md](LICENSE.md).

[dll]: https://github.com/ValveSoftware/openvr/raw/master/bin/win64/openvr_api.dll
