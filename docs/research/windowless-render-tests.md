# Windowless render tests: drawing the Pet into memory and checking its pixels

**Researched:** 2026-11-03, for Bevy 0.20.1, wgpu 30.0.1, rustc 1.97.1 and cargo-nextest 0.9.150 on Windows 11.
**Purpose:** find out whether tests can check the real Bevy drawing of the Pet while the owner works. The owner's rule
for background tests: never take screenshots, never show anything on screen, never touch the mouse or keyboard.

**How to read the marks.** Each fact has one:

| Mark | Meaning |
|---|---|
| **Verified** | Read in the crate source in the local cargo registry, which is what docs.rs builds from. |
| **Measured** | Run on this machine for this document. |
| **Inference** | My reasoning from verified or measured facts. Not run. |
| **Not verified** | I found no source that settles it, and didn't run it. |

`<reg>` means `C:\Users\flynn\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f`. Other paths are relative to the
repo root. Context7 wasn't used: the crate source is the source of truth for 0.20.1.

**The prototype** is in `C:\Users\flynn\AppData\Local\Temp\desktop-pet-windowless-render`, outside the repo. It uses
`bevy = "=0.20.1"` with the workspace's features, the repo's `Cargo.lock`, and the workspace's `[profile.dev]`, so its
timings are what `cargo nextest run` would build. Its `src/lib.rs` stands in for the app's engine and knows nothing about
tests. Everything test-side is in `tests/`. It also compiles the repo's real `engine/placeholder.rs` and
`engine/overlay.rs` unchanged, read through `#[path]`, with path dependencies on `pet-core` and `pet-platform`.

**Machine:** Ryzen 9 3950X with 32 logical processors and an RTX 2080 Ti on driver 32.0.16.1088, the same machine as
[simulation-architecture.md](simulation-architecture.md#drawing-cost-measured). The owner was using it throughout.

## Bottom line

- **It works.** A Bevy `App` with no window and no winit draws the Pet into an `Image` on DX12. The test reads the pixels
  back and compares every byte. All 960 pixels matched exactly, on the RTX 2080 Ti and on WARP, Windows' software
  renderer. The repo's real `PlaceholderPlugin`, compiled unchanged, also matched exactly, with its animation changing
  the pixels. Measured.
- **Nothing appeared on screen.** A watcher process hooked window creation before the test started and saw no window,
  not even a hidden one. Measured, with a control that proves the watcher sees hidden windows.
- **No screenshot, no input.** The pixels come from a GPU copy of the app's own texture, not from the screen. With winit
  gone, the app has no event loop and reads no OS input. Verified, and Inference for input.
- **Speed.** About 0.85 to 0.95 s to start, then about 5 ms per frame read back. A one-check test takes about 1 s as its own
  process. Measured.
- **Tests can run in parallel.** 32 copies passed at every level from 1 to 32 processes at once. Throughput stops
  improving at about 8 to 16, and each process takes about 326 MB in a debug build. Measured.
- **A busy GPU slows it but doesn't break it.** With another process holding the GPU at 99%, every test still passed with
  exact pixels on the same GPU, at about half speed per frame. Measured.
- **With no GPU, it runs on WARP.** Forcing the software adapter gave identical pixels at similar speed. On a machine
  with no GPU, wgpu's DX12 backend keeps WARP and Bevy picks it. Measured for WARP, Verified for how the adapter is
  picked, and Not verified on a real machine with no GPU.
- **It can run inside `cargo nextest run` as ordinary tests.** Measured in the prototype. Cap it with a nextest test
  group of about 8 to limit memory.
- **No test-only code in `src/` is needed.** The real `PlaceholderPlugin` already spawns no camera, and it drew
  correctly into a test's image camera. What's missing is access: `engine` is private, and some settings live only
  inside `engine::app()`. The fix is a split plus three exports. Measured for the plugin, Inference for the exported shape.
  See [Production code shape](#production-code-shape).

## Terms

| Term | Meaning |
|---|---|
| **Render target** | Where a camera's picture goes: a window, or an `Image` in GPU memory. |
| **Readback** | Copying a texture from GPU memory to CPU memory so code can read it. |
| **winit** | The crate that makes the OS window and runs its event loop for Bevy. |
| **WARP** | Windows Advanced Rasterization Platform, a DX12 driver that runs on the CPU. It ships with Windows as the "Microsoft Basic Render Driver". |
| **Message-only window** | A window that is never visible and exists only to receive messages. Libraries sometimes create them as hidden helpers. |
| **WinEvent hook** | A Windows request to be told when windows anywhere are created or shown. See [e2e-testing-and-fullscreen-events.md](e2e-testing-and-fullscreen-events.md#terms). |
| **Validation layer** | Extra checking that DX12 and wgpu do in debug builds. It catches mistakes but costs memory. |
| **TDR** | Windows resets a GPU driver that hangs for about 2 s. |

## 1. The 0.20.1 API

The template is Bevy's own example
[`externally_driven_headless_renderer.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/examples/app/externally_driven_headless_renderer.rs)
(`<reg>/bevy-0.20.1/examples/app/externally_driven_headless_renderer.rs`). It saves files to disk with `Screenshot`, so
the prototype uses `Readback` instead.

| Fact | Status | Source |
|---|---|---|
| The render target is its own component, `bevy::camera::RenderTarget`, spawned next to the camera. Its variants are `Window`, `Image(ImageRenderTarget)`, `TextureView` and `None`. | Verified | `<reg>/bevy_camera-0.20.1/src/camera.rs:893` |
| `RenderTarget::from(Handle<Image>)` exists and sets the scale factor to 1.0. | Verified | `camera.rs:1024` to `:1037` |
| `Image::new_target_texture(w, h, format, view_format)` sets TEXTURE_BINDING, COPY_DST and RENDER_ATTACHMENT, but **not COPY_SRC**. Readback needs COPY_SRC, so the test adds it. | Verified | `<reg>/bevy_image-0.20.1/src/image.rs:1253`, lines 1265 to 1267 |
| Readback is `bevy::render::gpu_readback::Readback::texture(handle)` on an entity. The bytes arrive as the `ReadbackComplete` entity event, read with an observer (`On<ReadbackComplete>`). `GpuReadbackPlugin` is part of `RenderPlugin`. | Verified | `<reg>/bevy_render-0.20.1/src/gpu_readback.rs:85`, `:157`; `bevy_render-0.20.1/src/lib.rs:416` |
| The copy is queued in the same frame's render, after the render graph. The buffer is mapped in Cleanup, and the event fires in the next frame's extract. So results arrive one update later. | Verified | `gpu_readback.rs:66`, `:72`, `:275`, `:401`; `<reg>/bevy_render-0.20.1/src/renderer/mod.rs:104` |
| The map callback only runs when the device is polled. The test waits with `RenderDevice::wgpu_device().poll(PollType::Wait { .. })` after each update, as the example does. | Verified | `renderer/render_device.rs:130`, the example |
| Each row of read-back data is padded to a multiple of 256 bytes. | Verified | `gpu_readback.rs:431`, `:436` |
| `ReadbackOnce` is removed in `First`, so if test code spawns it between updates it's gone before extract. Spawned that way, it never fired in 6 updates. | Verified, and Measured | `gpu_readback.rs:145` |
| With the `bevy_window` feature on, `DefaultPlugins` has no `ScheduleRunnerPlugin`. Once `WinitPlugin` is disabled, the runner is `run_once`, so the test drives the app with `App::update()`. | Verified | `<reg>/bevy_internal-0.20.1/src/default_plugins.rs:19`, `:40`; `<reg>/bevy_app-0.20.1/src/app.rs:152`, `:158` |
| Without `run()`, the test must call `app.finish()` and `app.cleanup()`. `RenderPlugin::finish` is what moves the GPU device into the render world. | Verified | `app.rs:261`, `:281`; `bevy_render-0.20.1/src/lib.rs:493` |
| `WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, .. }` keeps the window plugin, which much of Bevy expects, without making a window. | Verified | `<reg>/bevy_window-0.20.1/src/lib.rs:160`, the example |
| `RenderPlugin::synchronous_pipeline_compilation: true` compiles shaders before the first frame, so early frames aren't blank while they compile. | Verified | `bevy_render-0.20.1/src/lib.rs:164` |
| `Camera2d` brings `Tonemapping::Linear` and `DebandDither`, whose default is `Disabled`. So no dither noise. | Verified | `<reg>/bevy_core_pipeline-0.20.1/src/core_2d/mod.rs:52`, `:56`; `<reg>/bevy_render-0.20.1/src/view/mod.rs:351` |

**Harness setup**, in the prototype's `tests/common/mod.rs`:

- `DefaultPlugins` with `WinitPlugin` and `PipelinedRenderingPlugin` disabled, as the app does.
- `WindowPlugin` with no primary window.
- `RenderPlugin { synchronous_pipeline_compilation: true, ..render_plugin() }`, using the app's settings: DX12 and
  `MemoryHints::MemoryUsage`.
- `ClearColor(Color::NONE)` and `TimeUpdateStrategy::ManualDuration(Duration::ZERO)`.
- A 40 × 24 `Rgba8UnormSrgb` target. A row is 160 bytes, so the padding code really runs.
- A camera spawned as `(Camera2d, Msaa::Off, RenderTarget::from(image))`.

## 2. What the prototype checked

The art is 8 × 6 with two frames, opaque or fully clear, using the app's palette. It has clear corners and a gap between
the legs. It's drawn at 2× with a nearest-neighbour sampler, its top-left at (4, 6).

| Check | Result | Status |
|---|---|---|
| Every byte of all 960 pixels equals the art at 2×, with a transparent background. That covers crisp edges, scale and background in one comparison. | Passed, then repeated 50 times with identical bytes | Measured |
| After a 260 ms time step, the pixels equal frame 1 and differ from frame 0. | Passed | Measured |
| A frame change shows in the frame drawn by the same update. | Yes | Measured |
| Does update 1, which runs `Startup` and spawns the sprite, draw it? | No, it comes out blank. Update 2 is the first with the sprite, so the harness does one update before any check | Measured. Why: Not verified |
| sRGB textures in, an sRGB target out, `Tonemapping::Linear`: are the bytes unchanged? | Yes, on both RTX and WARP | Measured |
| Four apps on four threads in one process, each drawing and checking | All exact | Measured |
| The repo's real `PlaceholderPlugin`, unchanged, with `OverlayRect` and the work area both set to a 64 × 40 rect: all 2,560 pixels equal frame 0 at 2× at the bottom centre. The expected pixels come from its public `tray_icon_image()`. | Passed: 1,000 opaque pixels, 0 differences | Measured |
| The same plugin after a 260 ms step | Pixels change | Measured |
| The same, with `LogPlugin` kept in all four | Works. Three apps log an `ERROR` that the global logger is already set | Measured. Cause Verified at `<reg>/bevy_log-0.20.1/src/lib.rs:401` |

Exact equality depends on art that is fully opaque or fully clear, integer scale, even sizes and integer positions. Art
with partial alpha, or a position between pixels, would blend and round. Tests would then need expected pixels made the
same way, or a tolerance. Inference.

## 3. No window, seen from outside

**The watcher.** `window_watch.exe` installs an out-of-context WinEvent hook for `EVENT_OBJECT_CREATE` to
`EVENT_OBJECT_SHOW` across all processes. Only then does it start the test binary, and it keeps events from the child's
process id. It waits on the child's handle and its own message queue, so it never polls.

| Run under the watcher | Window events | Status |
|---|---|---|
| The three render tests: one image, frame change, four threads | **0**: nothing created, nothing shown | Measured |
| The two real-`PlaceholderPlugin` tests | **0** | Measured |
| Control: a test that creates one hidden window and one message-only window | 3 created (the two `Static` windows and the `IME` window Windows adds), 3 destroyed at exit, 0 shown | Measured |

**From inside.** At the end of the main test, the process listed its own windows with `EnumWindows`, including hidden
ones, and walked the message-only windows with `FindWindowExW(HWND_MESSAGE, ..)`. It found **none**. The same code in the
control test found the hidden `Static`, the `IME` window and the message-only `Static`. Measured.

A window can't be shown without first being created, so 0 create events rules out any shown window too. The control
can't include a shown window without breaking the owner's rule, so that reasoning is what covers showing.

So there are **no hidden helper windows**: DX12 with an image target needs none. Measured.

**Screenshots and input.**

- The pixels come from `copy_texture_to_buffer` on the app's own target texture (`gpu_readback.rs:367`). The screen is
  never read. Verified.
- With `WinitPlugin` disabled there is no event loop and no window to receive input. Bevy's `InputPlugin` only holds what
  winit would have fed it. Inference.

## 4. Timings

Debug build with the workspace profile, RTX 2080 Ti, the owner using the machine. **Measured.**

| What | Time |
|---|---|
| Startup, from `App::new()` to after the first update | 823 to 949 ms over about 40 runs one at a time, median about 860 |
| Of which adding plugins, which creates the DX12 device | 481 to 506 ms |
| Of which `finish` and `cleanup` | 0 to 1 ms |
| Of which the first update: `Startup` and synchronous shader compiles | 401 to 427 ms |
| First capture after startup: 2 updates | 146 to 152 ms |
| Each later capture: 2 updates, 2 GPU waits, readback, unpadding | p50 4.5 to 5.6 ms, p95 6.0 to 6.6 ms, max 7.0 ms |
| One update plus a GPU wait | 2.0 to 2.5 ms |
| A whole one-check test as its own process (nextest's figure) | 1.1 to 1.6 s |

**D3D12 validation is on in test builds.** wgpu turns on `DEBUG | VALIDATION` whenever `debug_assertions` is set
(`<reg>/wgpu-types-30.0.1/src/instance.rs:268`), and `WGPU_VALIDATION=0` turns it off (`instance.rs:308`). Verified.
The workspace's `[profile.dev.package."*"]` raises only the opt-level, so dependencies keep `debug_assertions`
([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html#debug-assertions)). The memory figures below
confirm validation is on. Inference, from the docs, and Measured.

| | Startup, 5 runs | Peak working set | Private memory |
|---|---|---|---|
| Validation on (default) | 838 to 887 ms | 309 MB | 326 MB |
| `WGPU_VALIDATION=0` | 774 to 844 ms | 151 MB | 170 MB |

Measured. Validation barely changes the time, but it doubles the memory.

## 5. Several at once

nextest runs each test in its own process. 32 identical one-check tests were run with `-j` set to each level. All 32
passed every time. **Measured.**

| `-j` | Whole run | Per test, median / max | Startup, median | Peak working set, median |
|---|---|---|---|---|
| 1 | 36.3 s | 1.02 / 1.10 s | 0.86 s | 306 MB |
| 8 | 8.0 s | 1.57 / 2.30 s | 1.39 s | 309 MB |
| 16 | 7.1 s | 2.61 / 4.09 s | 2.18 s | 318 MB |
| 32 | 8.0 s | 6.46 / 7.39 s | 6.02 s | 345 MB |

At `-j 16` with `WGPU_VALIDATION=0` the run took 6.0 s. Measured.

- Throughput stops improving between 8 and 16 processes. Past that, startup just slows down. Measured.
- At nextest's default here, 32 processes, the tests would take about 32 × 326 MB, roughly 10 GB of private memory.
  Inference, from the per-process figure.
- They don't need to run one at a time. A nextest test group with `max-threads` of about 8 is the sensible cap.
  Inference.
- Threads in one process also work (section 2). nextest doesn't use that, so it matters less. Measured.

## 6. A busy GPU, and no GPU

**Under heavy GPU load.** `gpu_hog.exe` is a windowless wgpu compute load. It submits back-to-back dispatches of about
30 ms each, far below the TDR limit, and `nvidia-smi` read 97 to 99% GPU use. With it running for 45 s:

| | Idle GPU | GPU at 99% |
|---|---|---|
| Startup | about 860 ms | 898 ms |
| Capture p50 / p95 | 4.5 / 6.0 ms | 9.2 / 10.6 ms |
| One update plus wait | 2.0 to 2.5 ms | 4.6 ms |
| 32 tests at `-j 8` | 8.0 s | 8.9 s |
| Pixels | exact | exact |
| Adapter | RTX 2080 Ti | RTX 2080 Ti, no fallback |

Measured. Windows shared the GPU between the two processes. Nothing failed, and nothing fell back to another adapter.

**WARP.** `WGPU_FORCE_FALLBACK_ADAPTER=1` (`<reg>/bevy_render-0.20.1/src/renderer/mod.rs:261`) made Bevy pick
`Microsoft Basic Render Driver (Cpu, Dx12)`. Measured:

- All three render tests passed with exact pixels, the four-thread test included.
- Startup 767 ms, first capture 159 ms, capture p50 5.9 ms, one update 3.3 ms. That's about the same as the GPU, because
  the scene is tiny.

**A machine with no GPU.**

- wgpu's DX12 backend lists the software WARP adapter. It drops only a second WARP entry that claims to be a real GPU
  (`<reg>/wgpu-hal-30.0.1/src/auxil/dxgi/factory.rs:35` to `:47`). Verified.
- wgpu ranks a CPU adapter last but still returns it when it's the only one (`<reg>/wgpu-core-30.0.1/src/instance.rs:571`
  to `:603`). Verified.
- So with no GPU the tests would run on WARP without any setting. Inference, from those facts and the forced-WARP run.
  Not verified on a real machine with no GPU, or on a CI runner image.
- If no adapter exists at all, Bevy panics with "Unable to find a GPU!" (`renderer/mod.rs:160`, `:308`). Verified.
- `render_plugin()` sets `backends: Some(Backends::DX12)` explicitly, so the `WGPU_BACKEND` variable has no effect.
  `WGPU_FORCE_FALLBACK_ADAPTER` and `WGPU_ADAPTER_NAME` still work, because Bevy reads them separately
  (`renderer/mod.rs:261`, `:266`). Verified.

**Device loss** in the middle of a test, such as a TDR, wasn't tried. Not verified.

## 7. Inside `cargo nextest run`

Yes, as ordinary tests. Measured in the prototype with nextest 0.9.150, at every `-j` from 1 to 32.

For the repo:

- A new test binary such as `crates/pet-app/tests/render.rs` would be picked up by the default profile. Its filter is
  only `not binary(e2e)`, in `.config/nextest.toml`. Verified.
- A test group caps memory, for example `gpu = { max-threads = 8 }` with an override for `binary(render)`. Inference,
  from section 5.
- At about 1 to 1.6 s per test process, 20 render tests would add about 3 to 4 s to a run at 8 at a time. Inference.
- They don't take the single-instance mutex, because they never call `pet_app::run`. So they can run while the real Pet
  is running. Inference, from `crates/pet-app/src/lib.rs`.

## Production code shape

**What already fits.** Verified, in `crates/pet-app/src/engine/`:

- `PlaceholderPlugin` spawns no camera. The camera comes from `overlay::spawn_camera`, added in `engine::app()`
  (`mod.rs`, `overlay.rs`). So the renderer already draws into whatever camera exists. Measured too: the unchanged
  plugin drew exactly into the prototype's image camera (section 2).
- `publish_hit_target` does nothing without a `PlatformHandle` (`placeholder.rs`), so the plugin runs with no platform.

**What stands in the way.** Verified:

- `engine` is a private module. `lib.rs` exports only `run` and `RENDER_FAILURE_EXIT`, so a test in `tests/` can't reach
  the plugin.
- `PlaceholderPlugin`'s `Startup` system needs `Res<OverlayRect>`, which only `engine::app()` inserts.
- Four things the drawing depends on are set up only inside `engine::app()`: `ClearColor(Color::NONE)`, `Msaa::Off` on
  the camera, the disabled `PipelinedRenderingPlugin`, and `render_config::render_plugin()`. A test would have to copy
  them, and the copies could drift from the app.

**The shape that needs no test-only code.** This is a split and some exports, not new behaviour:

```rust
// engine/mod.rs: everything public is used by the app itself
pub struct PetRenderPlugin { pub work_area: Rect, pub overlay: Rect }
// build(): inserts ClearColor(NONE) and OverlayRect(overlay), adds PlaceholderPlugin { work_area }

pub fn pet_camera() -> impl Bundle { (Camera2d, Msaa::Off) }  // overlay::spawn_camera spawns this

pub fn base_plugins() -> PluginGroupBuilder {
    DefaultPlugins.build()
        .disable::<PipelinedRenderingPlugin>()
        .set(render_config::render_plugin())
}

pub fn app(..) -> App {
    // base_plugins() + .set(LogPlugin { .. }) + .set(overlay::window_plugin(..))
    // + PetRenderPlugin + the overlay, presence and platform systems, as today
}
```

Then `lib.rs` exports `PetRenderPlugin`, `pet_camera` and `base_plugins`. The test-side harness lives in `tests/`:

1. `base_plugins()` with `WinitPlugin` and `LogPlugin` disabled.
2. `WindowPlugin { primary_window: None, exit_condition: DontExit, .. }`.
3. `RenderPlugin { synchronous_pipeline_compilation: true, ..render_plugin() }`. That means exporting `render_plugin`,
   or having `base_plugins` take the flag.
4. `PetRenderPlugin` with a small made-up work area and overlay, such as 40 × 24. Positions are relative to the overlay,
   so a small one is enough.
5. `(pet_camera(), RenderTarget::from(image))`, plus `TimeUpdateStrategy::ManualDuration` for frames.

The other option is unit tests inside `src/engine/`, like the existing ones in `placeholder.rs`. They can reach private
items without any exports, but that puts the harness in `src/`. Inference.

## Limits

- **The window path isn't covered:** swapchain, `CompositeAlphaMode::PreMultiplied`, transparency as the compositor
  shows it, click-through, Presence and the tray. Those stay in the e2e layer described in
  [e2e-testing-and-fullscreen-events.md](e2e-testing-and-fullscreen-events.md). Inference.
- **The output format differs.** The test target is `Rgba8UnormSrgb`. The window's swapchain format wasn't checked. The
  drawing up to the final write is the same. Not verified.
- **The readback lag.** Every check costs at least two updates, and the first update after startup is blank. See
  section 2. Measured.
- **Memory:** about 326 MB per test process with validation on. Measured.

## Not verified

- **Why the first update draws nothing,** even with synchronous shader compiles.
- **A real machine with no GPU,** or a Windows CI runner. Only forced WARP was run.
- **Device loss (TDR) during a test,** and what Bevy's default render error handler does then in a test.
- **The overlay window's swapchain format,** compared with the test's `Rgba8UnormSrgb` target.
- **Full-size targets.** Only 40 × 24 was read back. A 3840 × 2088 overlay is 32 MB per readback. Inference: slower in
  proportion, and not needed for checking the art.
- **Release-build timings.** Only the dev profile was measured, because that's what nextest builds.
