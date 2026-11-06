# The Pet's own window: Settings, Mind inspector, tuning and a Mind editor

**Researched:** 2026-11-06, for Bevy 0.20.1, wgpu 30.0.1, rustc 1.97.1 on Windows 11.
**Purpose:** choose how to build one user-facing window that opens from the tray. It hosts Settings (SET-01), a live
inspector of the Pet's Mind, balance tuning (PRD §8.2), and later a node-graph editor where users wire up the Mind and
plug in Actions from their Pack. Some sections may be dev-only. The window must cost as little as possible while closed.

**How to read the marks.** Each fact has one:

| Mark | Meaning |
|---|---|
| **Verified** | Read in crate source in `<reg>` (what docs.rs builds from), on crates.io, GitHub, or official docs. |
| **Measured** | Run on this machine for this document. |
| **Inference** | My reasoning from verified or measured facts. Not run. |
| **Not verified** | No source settled it, and I didn't run it. |

`<reg>` is `C:\Users\flynn\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f`. Context7 wasn't available in this
session, so crate source, crates.io and GitHub are the sources.

## Bottom line

- **Recommendation: a separate small window program written with egui, started by the Pet from the tray.** Use
  eframe pinned to egui 0.36 (eframe 0.36.2) so that `egui-snarl` 0.12.0 fits, with `egui_plot` for charts. It talks to
  the Pet over a pipe. **Inference**, from the measurements below.
  - The spike measured eframe 0.37 with the glow renderer. Snarl ran inside bevy_egui, which uses the same egui 0.36,
    but it wasn't built with eframe 0.36. **Not verified**
  - It is the only Rust option that meets "costs nothing when closed". When it exits, all its memory goes with it, and
    the Pet stays at its 96 MB baseline. **Measured**
  - It leaves the Overlay's engine alone: no second camera, no change to how Hidden works, and no DX12 risk. **Inference**
  - It costs **about 87 MB while open**, which brings Pet plus window to about 184 MB. The egui exe in the spike was
    6.2 MB. **Measured**
- **The runner-up is `bevy_egui` 0.43.1 in a second Bevy window, inside the Pet.** It works beside the transparent DX12
  Overlay, adds 7.4 MB to the exe, ran the snarl node graph, and reads the simulation directly. **Measured**
  - **It breaks "costs nothing when closed", as written.** The first open took the Pet up by 18–30 MB (113–125 MB),
    and closing gave none of it back.
  - **Every reopen added a little more**, and less each time: +3.7, +2.6 and +2.1 MB over four opens, ending at
    133 MB, and it held there for 55 s closed.
  - It stays under 150 MB for a while, but it creeps toward the limit over a long day of opening it. **Measured**
- **A WebView2 window (wry 0.57) costs the same 87 MB** as the egui exe, and gives the most polished, accessible UI. The
  price is a second tech stack (HTML and JS). It is the alternative if a branded look matters more than staying in Rust.
  **Measured**
- **Bevy's own UI (`bevy_ui` + feathers) works, but it costs the most.** It adds 24.5 MB to the exe and 42 MB of RAM
  that isn't given back. It has no chart widget and no node graph, and feathers says it isn't meant for end-user UI.
  **Measured / Verified**
- **Win32 dialogs and a page in the user's browser are out for the editor.** Win32 would need every node and chart
  drawn by hand in GDI. A browser tab can't be kept above the Overlay during an Encounter. **Inference**

- **If looks come first, the answer changes to a WebView2 window**, still as its own process. See
  [Visual options](#visual-options-for-an-owner-who-puts-looks-first). **Inference**

## Comparison

Release builds use the repo's `[profile.release]` and linker. The baseline Overlay exe is **39.15 MB**. RAM is private
working set, which Task Manager shows as "Memory". The baseline is about 96 MB, give or take 2 MB from run to run.

| | 1. bevy_egui, 2nd window | 2. bevy_ui + feathers, 2nd window | 3. eframe exe over a pipe | 4. Win32 window | 5a. Page served to a browser | 5b. Page in a WebView2 window (wry) |
|---|---|---|---|---|---|---|
| Exe size added | **+7.35 MB** (+0.21 MB more with snarl) | **+24.46 MB** | app +0.13 MB; panel exe 6.20 MB | +0.09 MB | +0.09 MB | app +0.09 MB; viewer exe 0.82 MB |
| Build time added (release) | 4 m 22 s, then +2 m 25 s for snarl | 6 m 17 s | panel exe 1 m 21 s clean | 1 m 38 s | 1 m 44 s | viewer exe 35 s clean |
| RAM open, Pet process | **+18.4 MB** (+27.5 MB with graph) | **+42.0 MB** | +0.7 MB | +0.5 MB | +0.7 MB | +0.7 MB |
| RAM open, panel's own processes | none | none | **87.5 MB** (1 process) | none | the browser's | **87.0 MB** (7 processes) |
| RAM after close | **stays**, +2–4 MB more per reopen | **stays** (+42 MB) | freed | freed | freed | freed |
| CPU open → after close (whole machine; base 0.53 %) | 1.39 % → 0.66 % | 1.82 % → 1.03 % | 1.11 % → 0.56 % | 0.89 % → 0.59 % | 0.55 % (app only) | 0.55 % + 0.37 % |
| Works beside the DX12 Overlay | **yes, Measured** | **yes, Measured** | yes, separate process | opened without error, but didn't come to the front | yes | **yes, Measured** |
| Can stay above the Overlay (Encounter) | yes (`WindowLevel`) | yes | yes (`with_always_on_top`) | yes (`WS_EX_TOPMOST`) | **no** | yes |
| Charts | `egui_plot` 0.37 | none: Nodes as bars, or gizmos | `egui_plot` 0.38 | GDI by hand | canvas or a JS library | canvas or a JS library |
| Sliders, text fields | yes | feathers slider and text input | yes | trackbar and edit control | HTML | HTML |
| Node-graph editor | **egui-snarl 0.12, works (Measured)** | none found | snarl only if eframe is pinned to egui 0.36 | none | JS libraries, not evaluated | JS libraries, not evaluated |
| End-user polish | themable, but "not native" by design | full styling, but you build the widgets | as 1 | native, dated | best (CSS) | best (CSS) |
| Accessibility, IME | AccessKit (opt-in feature), IME | AccessKit roles in widgets, IME | AccessKit, IME | native (UIA) | Chromium | Chromium |
| Coupling | reads ECS directly | reads ECS directly | wire format + process lifetime | unsafe code in pet-platform | wire format + local server | wire format + local server + WebView2 |
| Tied to the Bevy version | yes: 0.43.0 shipped a day after Bevy 0.20 | built in | **no** | no | no | no |

CPU was measured while other builds were running on the machine, so treat differences under about 0.3 % as noise.
"After close" CPU above the 0.53 % base is the closed panel's ongoing cost: plugin systems that still run every frame.
**Measured**

## How it was measured

A throwaway Cargo workspace in `%TEMP%\desktop-pet-panel-spike`, now deleted. It used the repo's `Cargo.lock`, Bevy
line, `[profile.*]` blocks and `rust-lld` linker. **Measured**

- **The Overlay copy.** A shared `overlay` crate copies `engine/overlay.rs` and `render_config.rs`. It has the
  transparent, undecorated, always-on-top, click-through window over the whole work area, and
  `CompositeAlphaMode::PreMultiplied`. It also has DX12, `MemoryHints::MemoryUsage`, MSAA off, no pipelined rendering,
  and a 30 fps reactive wait. It sets `WGPU_DX12_PRESENTATION_SYSTEM=Visual` as `pet_platform::prepare_process` does.
  Without that, DX12 offers only `Opaque` and the Overlay panics: "Requested alpha mode PreMultiplied is not in the
  list of supported alpha modes: [Opaque]". The Pet is a Mesh2d rectangle that moves every frame.
- **Fake Mind.** It has three drives with 60 values of history, five Action scores, the chosen Action, memory lines,
  and one balance value. It goes out as a read-only resource, and the panel sends back a `Tune` message.
- **One variant per option.** Each spike shows a chart of the drives, a score table, a slider that changes the balance
  value, and the memory lines (the egui and Win32 code is 164 and 220 lines). Option 3 starts `panel-exe` as a child
  and talks over its stdin and stdout. Option 5 is a `std`-only HTTP server on 127.0.0.1 that pushes snapshots with
  Server-Sent Events, with a `wry` 0.57.1 window as the viewer.
- **Timeline.** Overlay alone for 0–15 s, panel open for 15–40 s, closed for 40–62 s. `measure.ps1` read
  `Win32_PerfRawData_PerfProc_Process.WorkingSetPrivate` for the Pet and every process it started, at 12 s, 37 s and
  57 s. CPU is `TotalProcessorTime` over 10 s windows, divided by the 32 logical processors.
- **Reopen test.** `cycle.ps1` opened and closed the egui panel four times, 15 s each way, then left it closed for 55 s.
  It sampled every 5 s. The samples were 124.5, 128.2, 130.8 and 132.9 MB after each open, and 132.9 MB at the end.
- **Screenshots.** They confirmed the Overlay stayed transparent, with the desktop visible and the Pet drawn, while the
  egui, bevy_ui and WebView2 panels drew correctly. Closing despawns the panel window and the Overlay keeps running.
- **Build times.** The in-process variants built one after another into one copy of the baseline's target directory,
  so each time is roughly what that variant adds. bevy_ui came after egui, so its figure may be slightly low. Cargo's
  own `Finished … in` line is the time. A clean baseline build took 4 m 53 s. Touching `main.rs` and rebuilding took
  1 m 43 s: in release, any change to the app costs that relink, whichever UI it uses. The separate exes were clean
  builds in empty target directories. Other builds were running on the machine at the same time, so treat every time
  as ±30 %.
- **Not measured.** Closing with the window's X button (an automated close message was blocked by this session's
  permissions), DPI scaling above 100 %, a browser tab's own RAM, and eframe with its default wgpu renderer (the spike
  used glow).

## What each route needs from the app

**For the recommended separate window.** None of this was built beyond the spike's pipe. **Inference**

- **Starting it.** The Pet starts the window exe from the tray with piped stdin and stdout, as the spike did, and only
  ever one at a time.
- **Ending it with the Pet.** Put the child in a Windows job object, so it closes when the Pet does, even after a crash.
- **What goes out.** A `MindSnapshot` serialized with `serde` goes out a few times a second while the window is open,
  and nothing while it's closed.
- **What comes back.** `Tune` and Settings changes come back. The Pet stays the only writer of the save and settings
  files. The editor saves Pack files, and hot reload picks them up.
- **During an Encounter.** The window can use `with_always_on_top`, so it stays above the Overlay.

**If the window lives inside the app (the runner-up).** These come from reading the real code, and from the spike.

1. **Closing the panel would quit the Pet.** `presence::exit_when_close_requested` exits on *any*
   `WindowCloseRequested`. It must only exit for the Overlay, and close the panel otherwise. The spike does this.
   **Verified**
2. **The panel freezes while the Pet is Hidden.** `presence::apply_presence` sets `is_active = false` on *every* camera
   and switches the wait to `Duration::MAX`. A fullscreen game or a Manual hide would freeze the window. The panel camera
   must be left out, and the wait must stay at 30 fps while the panel is open. **Verified**
3. **The panel gets its own camera.** It needs `RenderTarget::Window(panel)`, `RenderLayers::layer(1)` so the Pet isn't
   drawn into it, and `order: 1`. Set `EguiGlobalSettings::auto_create_primary_context = false`, or egui attaches to
   the Overlay's camera, the first one spawned. **Verified / Measured**
4. **The Visual swapchain is process-wide.** It applies to the panel too. With an opaque panel window it worked.
   **Measured**
5. **The DX12 shader bug doesn't touch these renderers.** The egui shader uses no `instance_index` or `vertex_index`
   and draws instance 0 (`render/egui.wesl`, `render_pass.rs` L202). `bevy_ui_render`'s shaders use neither. The egui
   shader also carries the struct workaround for the DX12 push-constant bug,
   [bevy_egui#429](https://github.com/vladbat00/bevy_egui/issues/429) and
   [wgpu#5683](https://github.com/gfx-rs/wgpu/issues/5683), which was closed 2026-10-31. **Verified**
6. **A node graph has to sit in its own egui `Window` or panel.** Inside a hand-made background `Ui` it drew an empty
   canvas. In an `egui::Window` the nodes, pins and headers drew correctly. **Measured**
7. **The fullscreen rule leaves the panel alone.** A maximised panel stops at the taskbar, which the rule in
   `pet-core::fullscreen` counts as not fullscreen. A borderless panel filling the monitor would hide the Pet.
   **Verified**
8. **Bevy warns on close.** It logs "Skipped event … for unknown winit Window Id" when a window closes. This is harmless
   and known ([bevy#12261](https://github.com/bevyengine/bevy/issues/12261)). **Measured**

## Coupling: where it sits in the architecture

- **Inspector.** `pet-sim` publishes a read-only `MindSnapshot`: drives, each candidate's score and considerations, the
  choice and its reasons, recent memory, and XP. The panel reads it. **Inference**
- **Tuning.** The panel sends a `Tune` message, and `pet-sim` applies it at the start of a tick using `pet-core`'s
  rules. The renderer stays read-only: a small `engine/panel_link` plugin owns the pipe, and it isn't part of
  `RendererPlugin`. **Inference**
- **Shared types.** `MindSnapshot`, `Tune` and the Settings messages belong in a small crate that both exes use, with
  no Bevy in it. `pet-core` could hold them, or a new `pet-panel-protocol` crate. **Inference**
- **Editor.** The Mind's choices are Pack data, and Packs already hot-reload, and only once they validate. So the editor
  can simply save Pack files. It needs no live link at all. **Inference**
- **Out of process.** Options 3 and 5 turn the snapshot and the message into a wire format. `serde` is already a
  dependency, and the pipe can be the child's stdin and stdout or a named pipe. They also have to manage the second
  process's lifetime and errors. A pipe is safer than a localhost port. Any local program, or a web page through the browser, can reach the port, so it
  would need a token and a `Host` check. **Inference**
- **The upgrade risk.** `bevy_egui` pins one Bevy version per release, and its README table maps Bevy 0.20 to 0.43.
  It shipped 0.43.0 on 2026-10-09, a day after Bevy 0.20 (2026-10-08). With option 1, a Bevy upgrade waits for it.
  The separate egui exe has no Bevy in it, so it only waits for `egui-snarl` and `egui_plot` to follow egui.
  **Verified**

## Node-graph editors

Checked on crates.io and GitHub on 2026-11-06. **Verified**

| Crate | Version | egui | Fits egui 0.36 (bevy_egui 0.43, eframe 0.36)? | Status |
|---|---|---|---|---|
| [egui-snarl](https://crates.io/crates/egui-snarl) | 0.12.0 | ^0.36 | **yes, built and ran** | active: last release 2026-08-26, 606 stars, 45 open issues, typed nodes, serde |
| [egui_node_editor](https://gitlab.com/cyloncore/egui_node_editor) | 0.11.0 | ^0.36 | yes (not built) | small: 16 k downloads, updated 2026-08-21 |
| [egui-graph-edit](https://crates.io/crates/egui-graph-edit) | 0.9.1 | ^0.35 | **no**, needs a new release | small: 53 stars, last push 2026-07-23 |
| [egui_node_graph2](https://crates.io/crates/egui_node_graph2) | 0.7.0 | ^0.29 | no | stale: last push 2024-12-18 |
| bevy_ui | — | — | — | nothing found. `bevy_node_graph` 0.0.1 is a name reservation |

snarl's README warns that UI scaling "works with some artefacts" and that it isn't feature-complete.

## Polish, theming and accessibility for end users

- **egui.** Its README lists a "Native looking interface" as a non-goal. You can theme colours, rounding, shadows and
  widget looks through `Visuals`, and fonts through `Context::set_fonts`. Expect a clean tool look rather than a
  branded one. AccessKit works on Windows through `bevy_egui`'s `accesskit` feature, which is off by default. IME
  arrives from Bevy's `Ime` messages (`src/input.rs`). Bevy's AccessKit crash when a second window closes is macOS-only
  ([bevy#19424](https://github.com/bevyengine/bevy/issues/19424)). The spike's panel had `accesskit` on and closed
  cleanly. In eframe, `accesskit` is a default feature. The spike turned default features off, so the real window
  should turn it back on. **Verified / Measured**
- **bevy_ui.** `bevy_ui_widgets` are headless: "These widgets have no inherent styling". They carry AccessKit roles,
  and the text input reads IME. Feathers is the styled set, but its docs say it is "deliberately not intended" for game
  UI. A polished window means writing your own widget kit. **Verified**
- **Web and WebView2.** This route is the most polished, with full IME, keyboard and screen-reader support, plus CSS.
  The cost is a second tech stack (HTML, JS or TS) and the 87 MB process group. WebView2 is part of Windows 11, per
  Microsoft's [distribution docs](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
  **Verified / Measured**
- **Win32.** Native controls are accessible out of the box, but they look dated and offer nothing for charts or graphs.
  **Inference**

## Which parts can be dev-only, and how to hide them

- **Ship to everyone:** Settings, a friendly Mind view (current mood, what it's doing and why, recent memories), and
  later the Mind editor.
- **Dev-only:** raw utility scores and considerations, the balance sliders, formula replay, and the XP curve editor.
  **Inference**
- **The hiding mechanism.** Put them behind a cargo feature such as `dev-tools`, on both the window exe and `pet-app`.
  Use `#[cfg(feature = "dev-tools")]` on those sections and on the `Tune` handling, so a normal release build contains
  none of their code. This mirrors how the `e2e` feature hides the test binaries today. The owner runs
  `cargo run --release --features dev-tools`. A run-time switch would be the alternative, but it would ship the code
  to everyone. **Inference**

## Visual options: for an owner who puts looks first

Added 2026-11-06. The same "Pet panel" screen was built in four technologies, each with a deliberately playful theme
rather than its default one. Open [panel-visuals/index.html](panel-visuals/index.html) to see them side by side. Each
mockup has the tab bar, a drive chart, Action scores with reason chips, and a three-node graph.

- **No windows, no screenshots.** egui went through `egui_kittest` on wgpu, Slint through its software renderer into
  a buffer, and bevy_ui into an image with a GPU readback, as the windowless render tests do.
- **The HTML one is a live file** ([pet-panel.html](panel-visuals/pet-panel.html)) for the browser pane.
- **Fonts** are Bahnschrift and Segoe Print, which ship with Windows. Nothing was downloaded. **Measured**
- **Exe sizes** come from hello-world apps that were built but never run, since running them would open windows. For
  the same reason, Slint and Iced RAM wasn't measured.

| | egui 0.36 + snarl | HTML/CSS in WebView2 (wry 0.57) | Slint 1.18 | Iced 0.14 | Dioxus 0.7 | bevy_ui 0.20, own program |
|---|---|---|---|---|---|---|
| How styling works | paint anything with shapes, meshes and wgpu callbacks; widgets via `Visuals` | CSS, SVG, Canvas, WebGL | declarative markup with properties, gradients, shadows, `Path` | per-widget style functions, `Canvas`, `Shader` widget | HTML and CSS from Rust (`rsx!`) | nodes with gradients and shadows, `UiMaterial` shaders |
| Animation, transitions | by hand each frame (`animate_value_with_time`) | CSS transitions and keyframes, Web Animations | `animate`, `states`, transitions built in | `Animation` type in `iced_core` | CSS, plus Dioxus-Motion | by hand in systems, or in shaders |
| Node graph | **egui-snarl 0.12, ran** | JS libraries (not evaluated), or SVG by hand as in the mockup | `slint-node-editor` 2.0.0 (Sep 2026, 86 downloads) | `iced_nodegraph` 0.5.0 (461 downloads) | `dioxus-flow` 0.1.3 (567 downloads) | none |
| What is hard | every non-standard look is custom paint code; the text and layout engine is basic | a second language; the panel is web code talking over a pipe | software renderer drew path gradients flat and ignored `transform-rotation`; royalty-free licence needs attribution | small widget set; frequent breaking releases | native renderer (Blitz) is "still very young"; desktop is WebView2 anyway | no path drawing (lines in the mockup are rows of dots); every widget is yours |
| Designer workflow | none for styles; restart to see changes | browser devtools and any live-reload server | **live preview** with a data editor, VS Code extension, Figma plugin | time-travel debugger; Subsecond hot-patching | `dx serve` with Subsecond hot-patching | `hotpatching` feature (Subsecond) |
| Shipped apps showing the ceiling | [Rerun Viewer](https://app.rerun.io/) | new Microsoft Teams, built on WebView2 ([Microsoft Tech Community](https://techcommunity.microsoft.com/t5/microsoft-teams-blog/what-s-new-for-security-in-the-new-microsoft-teams/ba-p/3804261)) | [WesAudio plugin](https://wesaudio.com/product/_prometheus/), [Viiri Aava](https://viiri-audio.com) | [COSMIC desktop](https://github.com/pop-os/libcosmic), [Halloy](https://github.com/squidowl/halloy) | none found on its showcase pages | none known with a polished bevy_ui front end |
| Hello-world exe (release, repo profile) | 6.69 MB (eframe 0.36, glow) | 0.82 MB viewer + page | 10.79 MB (default winit + renderer) | 9.93 MB (default wgpu + tiny-skia) | 4.75 MB + WebView2 | 63.6 MB (with Overlay, earlier spike) |
| RAM while open | **≈ 87 MB** (eframe 0.37, measured) | **≈ 87 MB** (measured) | not measured | not measured | ≈ 87 MB (it is WebView2, Inference) | ≈ 96 MB baseline + UI (Inference, from the earlier spike) |

The "What is hard" row is my judgement (**Inference**), except the Slint renderer limits, which were **Measured**.

**Sources.** Slint features, licences and showcase: [slint.dev](https://slint.dev/). Royalty-free terms:
[Slint Royalty-free 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md),
which requires an `AboutSlint` screen or a badge on the download page. Iced: [README](https://github.com/iced-rs/iced)
and `iced_widget` 0.14.2 source (`canvas`, `shader`). Dioxus: [0.7 release](https://dioxuslabs.com/blog/release-070/).
egui: [0.36 README](https://github.com/emilk/egui/blob/0.36.0/README.md) and `egui-wgpu` 0.36 `CallbackTrait`.
Node-graph crates: crates.io on 2026-11-06. **Verified**

**What the mockups showed.** **Measured**

- **egui.** The Rust options reached the same playful look: egui by hand-painting nearly everything.
- **Slint.** It got there in about 200 lines of markup.
- **bevy_ui.** It also got there, but it needed dot chains for every curve.
- **HTML.** It was the only one where motion came almost free, and it animates: the chart scrolls, the face blinks,
  the wires flow, the cards spring in, and the nodes drag.

**Recommendation for looks and creative freedom: a WebView2 window, as its own small process.** **Inference**

- **Why.** It has by far the highest ceiling, and the best designer loop: devtools, live reload and CSS animation.
  Polish, IME and screen readers come built in.
- **No extra cost on top of the separate-window plan.** It needs the same pipe (`MindSnapshot` out, `Tune` in) and
  about the same RAM as the egui window, so the costs above already cover it.
- **What you take on.** A second language (HTML/CSS/TypeScript), a web build step, and a node graph built from a JS
  library or by hand.
- **The way to stay in Rust.** Dioxus desktop runs on the same WebView2, so it keeps CSS and costs the same RAM.
- **Runner-up for a Rust-only, designer-friendly route: Slint.** Its live preview and Figma plugin are the best Rust
  tooling. The costs are an attribution licence and a young node-graph crate.

## Open decisions

1. **Do the RAM and CPU budgets apply while the window is open?**
   - **RAM.** A separate process means about 184 MB while open, but nothing after. The in-process panel lands at
     113–125 MB and creeps up about 2–4 MB with each reopen, until restart. Task Manager's Processes tab groups child processes under the app and adds
     them up (**Not verified** on this machine), so the owner would see 184 MB.
   - **CPU.** While the window is open, every UI option goes past the < 1 % budget: 1.4 % for egui, 2.0 % with the
     graph, and 1.8 % for bevy_ui. With a separate window, that CPU is spent in the window's own process, not the
     Pet's. **Measured**
   - **The likely fix.** The inspector doesn't need 30 fps. Redrawing at a few frames a second, and at once on input,
     should bring that down. **Inference**, not tested.
2. **Would you rather keep it inside the app?** It's simpler: one exe, and no wire format. It's only acceptable if
   the retained 18–40 MB and the creep are fine. If so, one more spike should first find what holds that memory: wgpu
   buffers, the egui font atlas or pipeline caches (**Not verified**).
3. **Should it stay above the Overlay in an Encounter?** The Overlay catches every click on the Home monitor then. A
   normal window under it can't be clicked. Make the window always on top, or don't open it during a fight.
4. **The egui look, or a web look?** For a branded, end-user editor, WebView2 is the more polished option, at 87 MB
   while open and a second tech stack. Keeping the panel's code apart from the Bevy engine, talking only through
   `MindSnapshot` and `Tune`, keeps that move open.
5. **Is the `dev-tools` feature name and split right?** Which sections ship to everyone?

## Sources

- bevy_egui 0.43.1: [README version table](https://github.com/vladbat00/bevy_egui/blob/v0.43.1/README.md),
  [CHANGELOG](https://github.com/vladbat00/bevy_egui/blob/v0.43.1/CHANGELOG.md),
  [Cargo.toml features](https://github.com/vladbat00/bevy_egui/blob/v0.43.1/Cargo.toml), the multi-viewport issue
  [#454](https://github.com/vladbat00/bevy_egui/issues/454) (egui's own pop-out viewports aren't supported, so use Bevy
  windows).
- [egui_plot 0.37.0](https://crates.io/crates/egui_plot/0.37.0) (egui ^0.36),
  [egui 0.36 README](https://github.com/emilk/egui/blob/0.36.0/README.md),
  [egui Visuals](https://docs.rs/egui/0.36.2/egui/style/struct.Visuals.html),
  [eframe 0.37 README](https://github.com/emilk/egui/blob/0.37.0/crates/eframe/README.md) (wgpu is the default renderer).
- [Bevy 0.20 release notes](https://bevy.org/news/bevy-0-20/) (2026-10-08),
  [bevy_feathers 0.20.1](https://docs.rs/bevy_feathers/0.20.1/bevy_feathers/),
  [bevy_ui_widgets 0.20.1](https://docs.rs/bevy_ui_widgets/0.20.1/bevy_ui_widgets/). The `bevy_feathers` feature also
  needs `custom_cursor`, or `bevy_winit` 0.20.1 fails to compile (`CursorIcon::Custom` not covered). **Measured**
- [egui-snarl README](https://github.com/zakarumych/egui-snarl/blob/main/README.md),
  [wry 0.57.1 README](https://github.com/tauri-apps/wry/blob/wry-v0.57.1/README.md),
  [Tauri process model](https://v2.tauri.app/concept/process-model/),
  [WebView2 process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model).
- Repo: `crates/pet-app/src/engine/presence.rs`, `overlay.rs`, `render_config*.rs`, `pet-platform/src/windows/mod.rs`,
  `pet-core/src/fullscreen.rs`, PRD §6.2, §8.2 and SET-01.
