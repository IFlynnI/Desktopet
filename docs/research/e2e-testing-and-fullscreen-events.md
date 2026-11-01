# End-to-end testing and event-driven fullscreen detection

**Researched:** 2026-10-31, for Bevy 0.20.1, winit 0.30.13, wgpu 30, `windows` 0.62.2 on Windows 11.
**Purpose:** decide how to test the real app with as little manual checking as possible, and how Auto-hide
(WIN-07) should detect fullscreen apps without polling.

**How to read the citations.** Links are the sources. `<reg>` means
`C:\Users\flynn\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f`, and other `file:line` references are
relative to the repo root, with `RESULTS.md` and `win32.rs` short for the copies in `spikes/bevy-overlay/`. **UNCONFIRMED** means I found no source that settles it, and "Inference" means
my reasoning from a cited fact. Crate versions are from the crates.io API on the research date.

**Local measurements.** The first research pass couldn't run probes on this machine. P1, P3, P4, P5 and most of P6
have run since, on 2026-11-01. Their results are in [Measured on this machine](#measured-on-this-machine) and they
replace anything older in this document that disagrees. P2 and P7 are still open: see [Unconfirmed items](#unconfirmed-items).

## Bottom line

- **Nothing does what Playwright does for this app.** The closest equivalent is three layers: pure and
  in-process Bevy tests, a Rust harness that drives the real exe through Windows APIs, and `bevy_remote` for
  reading app state, compiled into test builds only.
- **Only three checks stay human:** unlocking the lock screen, sleep and resume on a normal desktop, and
  judging how the art looks.
- **Fullscreen detection:** the shell's `ABN_FULLSCREENAPP` message and a foreground hook on the shell thread,
  re-checked by a slow 2 s timer. The test is the shell's own rule: the foreground window's client area covers the
  Home monitor. See [Recommended design](#recommended-design), which was revised after the probes.

## Terms

[win32-and-rust.md](win32-and-rust.md) explains handles, messages, message loops and callbacks.

| Term | Meaning |
|---|---|
| **API** | A set of functions a program can call. |
| **HWND** | The number Windows uses to identify a window. |
| **Foreground window** | The window the user is working in now. It receives keyboard input. |
| **Client area** | The part of a window the app draws itself, without the title bar and borders. |
| **Hook** | A request to be told when something happens elsewhere on the system. |
| **WinEvent** | Accessibility notifications from Windows, such as "the foreground window changed". |
| **Out of context** | The hook callback runs in our process, and Windows copies each event across to us. Nothing is loaded into other apps. |
| **Elevated, integrity level** | Windows ranks processes by trust. "Run as administrator" gives a higher level. |
| **UIPI** | User Interface Privilege Isolation: Windows stops lower-level processes from driving higher-level ones. |
| **UI Automation (UIA)** | The Windows API that screen readers and test tools use to walk a tree of UI elements. |
| **ECS** | Entity Component System, how Bevy stores game state. |
| **BRP** | Bevy Remote Protocol: lets another process read and change a running app's ECS state. |
| **Headless** | Running with no window. |
| **Cargo feature** | A compile-time switch. Code behind a feature that's off isn't in the binary. |
| **DirectComposition** | A way of handing a window's pixels to the Windows compositor. The Overlay uses it (RESULTS.md:53). |
| **Exclusive fullscreen** | A Direct3D game takes over the display directly, not through the desktop compositor. |
| **TDR** | Timeout Detection and Recovery: Windows resets a GPU that stopped responding. |
| **Secure desktop** | The separate desktop for the lock screen and UAC prompts. Normal apps can't reach it. |
| **Rude window** | Microsoft's informal term for a window the taskbar treats as fullscreen. |

# Topic 1: is there a Playwright for this?

## Why Playwright's model doesn't carry over

Playwright works because the browser has a tree of elements. A
[locator](https://playwright.dev/docs/locators) finds an element in that tree, and
[auto-waiting](https://playwright.dev/docs/actionability) retries until it's visible, stable, enabled and
"receives events". This app has none of that:

- **No element tree.** Bevy draws pixels on the GPU. Windows sees one window and has no idea a Pet exists.
  UIA ["is an object model for UI objects"](https://devblogs.microsoft.com/oldnewthing/20250929-00/?p=111637),
  so anything that isn't one is invisible to it.
- **The important answers are outside the app.** "Did the click reach the app underneath?" is decided by
  Windows routing, so only Windows can answer it.
- **No isolation.** There's one real cursor and one screen, so tests can't run in parallel.

| Playwright idea | Closest equivalent here |
|---|---|
| Locator | BRP `world.query` for app state, `FindWindow`-style lookup and window styles for OS state |
| Auto-waiting | A small `wait_until(timeout, check)` helper. Nothing provides this for you. |
| "Receives events" check | `WindowFromPoint`, as the spike's self-test does (`win32.rs:442`) |
| "Stable" check | Pause the Pet first, or click tests flake on a moving target. The spike used a `frozen` flag (`win32.rs:40`). Here it would be a resource the harness sets through BRP. |
| `click()` | `SendInput` |
| Screenshot assertion | Capture a screen region and compare with a tolerance (`win32.rs:281`, `win32.rs:324`) |

## Bevy options

**In-process headless tests are the cheapest layer.**

- `MinimalPlugins` holds only task pools, frame count, time and a schedule runner
  (`<reg>/bevy_internal-0.20.1/src/default_plugins.rs:163`). `App::update` runs one frame
  (`<reg>/bevy_app-0.20.1/src/app.rs:158`).
- `TimeUpdateStrategy::ManualDuration` makes time deterministic (`<reg>/bevy_time-0.20.1/src/lib.rs:125`).
- Bevy's own example fakes input and windows this way (`<reg>/bevy-0.20.1/tests/how_to_test_apps.rs:1`).
- Limit: no real window, OS or GPU. This covers app logic and nothing about desktop behaviour.

**`CiTestingPlugin` is the first-party way to script a real run.** Behind the `bevy_ci_testing` feature
(`<reg>/bevy-0.20.1/Cargo.toml:2785`), it reads a RON file of frame-numbered events: screenshot, exit, custom
message (`<reg>/bevy_dev_tools-0.20.1/src/ci_testing/mod.rs:18`, `ci_testing/config.rs:37`). Bevy
[warns](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_dev_tools/src/ci_testing/mod.rs) to keep it
out of production builds. Its screenshots use Bevy's `Screenshot` component
(`<reg>/bevy_render-0.20.1/src/view/window/screenshot.rs:84`), which reads the frame inside the app. That
checks the art, not whether the compositor shows it as transparent.

**`bevy_remote` (BRP), read from the v0.20.1 tag:**

| Question | Finding |
|---|---|
| What it exposes | [22 method constants](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/builtin_methods.rs): query, get, insert, mutate and remove components and resources, spawn and despawn, trigger events, write messages, list schedules, watch for changes. You can add [custom methods](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/lib.rs). |
| Works on a running real app? | Yes. It's a [plugin inside the app](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/lib.rs). |
| Transport | [JSON-RPC 2.0 over HTTP](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/http.rs), default `127.0.0.1:15702`, with the listener on Bevy's IO task pool. |
| Security | The [HTTP plugin's source](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/http.rs) has no authentication. Inference: any local process could read and change state, one more reason to keep it out of release builds. |
| Type requirements | Requests name components by type and need reflection ([lib.rs](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/src/lib.rs)), so test-visible state must derive `Reflect`. |
| Memory and CPU cost | **UNCONFIRMED.** No published figures. It [adds hyper and async-io](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_remote/Cargo.toml) and runs systems in a new `RemoteLast` schedule every frame. |
| Compile out of release? | Yes. It's an optional feature (`<reg>/bevy-0.20.1/Cargo.toml:2817`), so put it behind our own `e2e` feature. |

Two consequences for this project:

- Requests are processed in `RemoteLast`, so they only run when the app updates. Inference: while the engine
  is paused during Hidden ([proposal](../proposals/item-2-architecture.md), line 81), BRP calls will stall.
  Assert Hidden from Windows with `IsWindowVisible`, not BRP.
- Budget runs (CPU, memory) must use a build without `e2e`, or they measure the wrong binary.

**Third-party Bevy test crates.** Few support 0.20, and none does what we need:

| Crate | Version | Supports 0.20.1? | Note |
|---|---|---|---|
| [`bevy_brp_extras`](https://github.com/natepiano/bevy_brp/blob/main/extras/README.md) | 0.23.0 | Yes, per its table | Adds screenshot and mouse and keyboard methods. Its input code uses Bevy's `MouseButtonInput` messages, and the repo has no `SendInput`, so it [never reaches Windows](https://github.com/natepiano/bevy_brp/tree/main/extras/src/mouse). It can't test click-through or focus. |
| [`bevy_headless_test`](https://github.com/warmar94/bevy_headless_test) | 0.2.0 | No | Pins `bevy = "=0.20.0"`. An [`=` requirement is exact](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html), and the proposal pins `=0.20.1`, so the two can't resolve together. |
| [`bevy_gpu_test`](https://github.com/peterellisjones/bevy_gpu_test) | 0.4.0 | Yes (`^0.20.0`) | Tests compute shaders only. Not relevant. |
| [`bevy_mod_screenshot_test`](https://github.com/azureblaze/bevy_mod_screenshot_test) | 0.2.0 | No (`^0.19`) | |
| [`rmv-bevy-testing-tools`](https://github.com/rmvermeulen/rmv-bevy-testing-tools) | 0.11.0 | No (`^0.19.1`) | |

## Windows options

**UI Automation tools add little here.** All of them read the same UIA tree:

| Tool | State |
|---|---|
| [`uiautomation`](https://github.com/leexgone/uiautomation-rs) (Rust) | 0.25.1, September 2026, maintained. |
| [WinAppDriver](https://github.com/microsoft/WinAppDriver) | Last release v1.2.1, November 2020. Appium's README says it ["has not been maintained by Microsoft for years"](https://github.com/appium/appium-windows-driver). |
| [Appium Windows Driver](https://github.com/appium/appium-windows-driver) | v6.3.1, and its README marks it deprecated. |
| [Appium WinCore Driver](https://github.com/appium/appium-wincore-driver) | v3.3.2, October 2026. The official replacement. Needs the Node-based Appium stack. |
| [NovaWindows Driver](https://github.com/AutomateThePlanet/appium-novawindows-driver) | Active, community-maintained, PowerShell backend. |

- **Can they see the Overlay?** The UIA tree's root has [child elements for application windows](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-treeoverview).
  I found no rule excluding tool windows, but whether a transparent `WS_EX_NOACTIVATE` window appears is
  **UNCONFIRMED** (probe P2). Even if it does, UIA would show one rectangle. It can't see the Pet or the Hit mask.
- **Can they see the tray?** Partly. Chen confirms UIA can enumerate notification icons, but
  [only ones actually on screen](https://devblogs.microsoft.com/oldnewthing/20250929-00/?p=111637), so overflow
  icons need the flyout opened first. He adds "there is no supported object model for notification icons".
- **Accessibility route.** The spike already pulls in `accesskit_windows` (`spikes/bevy-overlay/Cargo.lock:53`).
  Inference: adding an accessibility node would give UIA a Pet element, but it adds nothing BRP doesn't.

**Input synthesis with `SendInput`:**

- **Mouse: focus doesn't matter.** Windows normally sends mouse messages to
  [the window containing the cursor](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-mouse-input).
  `SendInput` [inserts events into the input stream](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput),
  so Inference: a synthetic click takes the same path as a real one.
- **Keyboard: focus matters.** Keys go to the [thread with keyboard focus](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input).
- **Click-through windows.** Windows [passes mouse events to the windows underneath](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)
  a layered `WS_EX_TRANSPARENT` window. Inference: synthetic clicks follow the same rule, which is the behaviour
  we want to test.
- **UIPI.** Input can only go to apps at [an equal or lower integrity level](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput),
  and "neither GetLastError nor the return value will indicate the failure". Run the harness unelevated, and make
  a test-owned window the foreground first. Otherwise an elevated window could silently swallow input.
- **Stuck keys.** `SendInput` [doesn't reset the keyboard state](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput),
  so a key the owner is holding interferes. Nobody can touch the machine during a run.
- [`enigo`](https://github.com/enigo-rs/enigo) 0.6.1 is a [wrapper around `SendInput`](https://github.com/enigo-rs/enigo/blob/main/src/win/win_impl.rs).
  We already use the `windows` crate, so calling it directly avoids a dependency.

**Asserting real OS outcomes:**

- `WindowFromPoint` is documented to [skip hidden and disabled windows](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-windowfrompoint).
  Microsoft doesn't say it skips `WS_EX_TRANSPARENT` windows. That behaviour is the spike's own measurement:
  224 points, 0 misrouted (RESULTS.md:21). It's also weaker than a real click, because it asks where a click
  would go without sending one.
- `GetWindowLongPtrW` reads window styles (`win32.rs:83`).
- `GetForegroundWindow` proves focus [before and after a click](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow). It can return NULL briefly.

**Screen capture:**

| Method | Finding | Use |
|---|---|---|
| GDI `BitBlt` + [`CAPTUREBLT`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt) | The spike's capture (`win32.rs:287`) does include the DirectComposition Overlay: the Pet is visible in `spikes/bevy-overlay/results/smoke-dx12-visual/shot_pet_startup.png` (inspected for this research). So the spike's transparency check is meaningful. | **Use this.** Move it to the harness. |
| [`xcap`](https://github.com/nashaofu/xcap) 0.9.8 | Its default monitor capture is [GDI `BitBlt` with `SRCCOPY` only](https://github.com/nashaofu/xcap/blob/master/src/windows/gdi.rs), no `CAPTUREBLT`. Whether that includes the Overlay is **UNCONFIRMED**. Its window listing [skips tool windows with an empty title](https://github.com/nashaofu/xcap/blob/master/src/windows/impl_window.rs), so it may not list the Overlay at all. An optional `wgc` feature exists. | No need to add it. |
| DXGI Desktop Duplication | [`DuplicateOutput`](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutput1-duplicateoutput) fails on the secure desktop unless running as LOCAL_SYSTEM, and allows four processes per session by default. | Overkill here. |

## Rust test tooling

- **`cargo test` integration tests** are the base. Cargo sets
  [`CARGO_BIN_EXE_<name>`](https://doc.rust-lang.org/cargo/reference/environment-variables.html) in tests under
  `tests/`, and builds the binary automatically.
- **`assert_cmd`** 2.2.2 is [aimed at command-line programs](https://github.com/assert-rs/assert_cmd) that run and
  finish. The Pet never exits, so `std::process::Command` plus a guard is enough.
  [`Child` doesn't kill its process when dropped](https://nexte.st/docs/features/leaky-tests/), so the guard must
  kill it and put the cursor back.
- **`cargo-nextest`** 0.9.150 (not installed here) suits this best.
  [Test groups with `max-threads = 1`](https://nexte.st/docs/configuration/test-groups/) force serial runs, and
  [`terminate-after`](https://nexte.st/docs/features/slow-tests/) kills a hung test. It runs each test in its own
  process, so in-process mutexes don't work. That rules out relying on `serial_test` 4.0.1 under nextest.
- **Without nextest:** `cargo test --test e2e -- --ignored --test-threads=1` also works.
- **Image comparison.** [`insta`](https://github.com/mitsuhiko/insta) 1.49.0 has `assert_binary_snapshot!`, but I
  found no pixel tolerance in it (**UNCONFIRMED**, comparator not read). Use it for text snapshots of state. For
  images, [`image-compare`](https://github.com/ChrisRega/image-compare) 0.5.0 or
  [`dify`](https://github.com/jihchi/dify) 0.8.0 exist, but the spike's 8-line `similarity` (`win32.rs:324`) is
  enough to start.

## Can a CI runner do this?

The working premise was that GitHub-hosted Windows runners have no real interactive desktop or NVIDIA GPU. The
evidence is more nuanced:

- **Interactive desktop: probably yes on x64.** A runner-images maintainer
  [reported](https://github.com/actions/runner-images/issues/14049) `UserInteractive=True` on `WinSta0` for
  `windows-11-arm`, where a process still can't take the foreground. That issue says the same test passes on
  `windows-latest`. This is a maintainer comment, not GitHub documentation, and I found no official statement.
- **Bevy runs real examples there.** Its [`windows-latest` DX12 job](https://github.com/bevyengine/bevy/blob/v0.20.1/.github/workflows/example-run.yml)
  runs examples with `bevy_ci_testing`. Those screenshots are read back inside the app, so this does **not** show
  that transparency works on a hosted runner.
- **NVIDIA GPU: no on standard runners.** [GPU runners](https://docs.github.com/en/actions/reference/runners/larger-runners)
  (Tesla T4, Windows listed) are ["larger runners"](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners)
  for Team and Enterprise Cloud plans only.
- **Hosted limits.** Jobs stop at [6 hours](https://docs.github.com/en/actions/reference/limits), which rules out
  a working-day soak.
- **Self-hosted on a real machine is the honest option for desktop tests.** Microsoft's Azure Pipelines guide says
  desktop UI tests need an agent [running interactively with autologon, not as a service](https://learn.microsoft.com/en-us/azure/devops/pipelines/test/ui-testing-considerations),
  and that disconnecting Remote Desktop locks the machine and can fail tests. Inference: the same applies to a
  GitHub self-hosted runner.

**So:** run pure and in-process tests on `windows-latest`. Run desktop tests locally on the owner's machine, or
later on a self-hosted interactive runner. Whether the Overlay works on a hosted runner is **UNCONFIRMED** (P7).

## Check by check

"Real exe" means a harness in `tests/` driving the actual binary. The CI column is for GitHub-hosted runners.

| Check | Where | How to assert it | Hosted CI? |
|---|---|---|---|
| Hit mask, Presence and fullscreen rules | **In-process** | Pure `pet-core` tests | Yes |
| Clicks reach the app underneath | **Real exe** | Harness opens its own plain window under the Pet, `SendInput` clicks a transparent pixel, that window records `WM_LBUTTONDOWN`. Clicking an opaque pixel must not reach it. | Unknown (P7) |
| Doesn't steal focus | **Real exe** | Harness window is foreground, click an opaque Pet pixel, `GetForegroundWindow` is unchanged. Closes the gap at RESULTS.md:22. | Unknown |
| Hidden from taskbar and Alt-Tab | **Real exe** | Assert `WS_EX_TOOLWINDOW` set, `WS_EX_APPWINDOW` clear, both at startup and after a while, because winit reset styles once (RESULTS.md:59). [Microsoft documents](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles) that style as hiding from both. | Likely, only reads styles |
| Tray menu | **Real exe** plus **in-process** | Menu actions: in-process. Real click: `tray-icon`'s [`rect()`](https://github.com/tauri-apps/tray-icon/blob/dev/src/platform_impl/windows/mod.rs) gives the icon position via `Shell_NotifyIconGetRect`, then `SendInput`. Measured 2026-11-01: for a new icon behind the ^ arrow, that position is the arrow's, so a click there opens the flyout, not the icon's menu. Also measured: the call works while the app runs and fails after it exits, even with the app's icon removal deliberately skipped. So it can show an icon exists, but not that it was removed cleanly. `NIM_MODIFY` behaves the same way. | Unknown |
| Explorer restart (`TaskbarCreated`) | **Real exe** | The shell [broadcasts `TaskbarCreated`](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar), and `tray-icon` [handles it](https://github.com/tauri-apps/tray-icon/blob/dev/src/platform_impl/windows/mod.rs). **The true check is an opt-in real restart**: kill and restart Explorer, then assert the icon's rect comes back. It's opt-in because it closes the owner's Explorer windows. Posting the message without a restart only shows the handler runs without crashing, because the icon was never removed, so that version can't catch a broken re-add. | No |
| Hotkey | **Real exe** plus **in-process** | Toggle logic: in-process. Registered: a second `RegisterHotKey` for Ctrl+Alt+P [typically fails](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey), which needs no synthetic keys. Delivery: `SendInput`, but see below. | Unknown |
| Auto-hide | **Real exe** plus **in-process** | Decision: pure tests. Real: harness opens a borderless monitor-sized window, gives it foreground by clicking it, asserts `IsWindowVisible` false, closes it, asserts true. Add a borderless window that resizes to fullscreen while already foreground, to cover the no-foreground-change case. | Unknown |
| Transparency | **Real exe** | Capture a region with the Overlay hidden and shown (`win32.rs:337`). Keep the caveat at RESULTS.md:71: it can't tell over black content. Use a harness window with a known colour behind it. | Unknown (P7) |
| Sleep and resume | **Manual** on a normal desktop | [`pwrtest /sleep`](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pwrtest-sleep-scenario) automates sleep, but [needs the WDK, an elevated prompt and a provisioned test computer](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pwrtest). Automatable later on a dedicated machine. RESULTS.md:73 says this is untested. | No |
| Lock screen | **Manual** for the unlock | `LockWorkStation` can start it, but ["to unlock the workstation, the user must log in"](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-lockworkstation). The harness can lock, wait for the human to unlock, then assert automatically. Session handling logic: in-process. | No |
| Forced GPU reset | **Real exe**, opt-in | [`dxcap -forcetdr`](https://learn.microsoft.com/en-us/windows/uwp/gaming/handling-device-lost-scenarios) forces a TDR, and needs the [Graphics Tools optional feature](https://learn.microsoft.com/en-us/windows/uwp/gaming/use-the-directx-runtime-and-visual-studio-graphics-diagnostic-features). See below for the expected result. Inference: it resets the whole GPU, so other apps are affected. Whether it needs elevation is **UNCONFIRMED**. | No |
| Multi-hour memory run | **Real exe**, opt-in | Release build without `e2e`. An external sampler reads the process's private working set on a timer, the measurement `win32.rs:155` makes today, but from outside. Needed for the slow creep at RESULTS.md:42. | No |

**Hotkey delivery, measured (P3, 2026-11-01).** `RegisterHotKey`'s docs say Windows matches
["when a key is pressed"](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey),
without covering injected keys. On this machine, keys sent with `SendInput` did trigger it: Ctrl+Alt+F24 and
Ctrl+Alt+P each produced exactly one `WM_HOTKEY`, so the hotkey test can use real delivery. Ctrl+Alt+P was free, and a
second registration of a taken combination failed with "Hot key is already registered" (`0x80070581`).

**GPU reset expectation.** Bevy's default handler
[quits the app on any render error](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/src/error_handler.rs)
(`<reg>/bevy_render-0.20.1/src/error_handler.rs:69`, `:79`). So a default build exits after a TDR. That fits the
proposal's `relaunch.rs` ("start a fresh process, at most 3 times in 10 minutes"), and the test should assert a new
process appears with a working Overlay. A custom handler returning `RenderErrorPolicy::Recover` (`:20`) is the
alternative.

**What stays on the manual list:** unlocking after a lock, sleep and resume on this machine, and subjective
judgement of the art. Everything else can be automated, with GPU reset and a true Explorer restart run only on
request because they disturb the desktop.

## Recommended stack

1. **`pet-core`: plain `cargo test`.** Pure rules, Given-When-Then, no Windows or Bevy.
2. **`pet-app` Bevy logic: in-process.** `App::new()`, `MinimalPlugins`, `ManualDuration`, manual `app.update()`.
3. **Real exe: harness in `crates/pet-app/tests/e2e/`.** `std::process::Command` with `CARGO_BIN_EXE_*`, the
   `windows` crate for `SendInput`, `WindowFromPoint`, window styles and `BitBlt`, and a kill-and-restore guard.
   The spike's metrics, capture and self-test [already move here](../proposals/item-2-architecture.md).
4. **State reading: `bevy_remote` behind an `e2e` cargo feature**, for "where is the Pet and which frame is it on",
   so the harness knows which pixels are opaque. Not for Hidden assertions.
5. **Runner: `cargo-nextest`** with one serial group, excluded from the default run:

   ```toml
   # .config/nextest.toml
   [profile.default]
   default-filter = "not binary(e2e)"

   [test-groups]
   desktop = { max-threads = 1 }

   [profile.e2e]
   default-filter = "binary(e2e)"
   slow-timeout = { period = "60s", terminate-after = 2 }

   [[profile.e2e.overrides]]
   filter = "binary(e2e)"
   test-group = "desktop"
   ```

6. **Release guard.** A CI step fails if `bevy_remote` appears in the release dependency tree. Screen capture lives
   only in the test binary. That keeps the shipped exe within SEN-02, which forbids screenshots.
7. **CI:** layers 1 and 2 on `windows-latest`. Layer 3 locally, hands off the mouse and keyboard, unelevated.

The `wait_until` helper, the harness's own plain window and the external memory sampler are ours to write.

# Topic 2: event-driven fullscreen detection

## How `SetWinEventHook` behaves

- **Thread.** The calling thread ["must have a message loop"](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook),
  and out-of-context events arrive on that same thread. The planned [shell thread](../proposals/item-2-architecture.md)
  already has one. [komorebi](https://github.com/LGUG2Z/komorebi/blob/master/komorebi/src/winevent_listener.rs)
  (Rust, `windows` crate) does the same on a dedicated thread.
- **Unhooking** [must happen on the thread that installed the hook](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwinevent),
  and it happens automatically when that thread ends.
- **Delivery.** Events are [queued and asynchronous](https://learn.microsoft.com/en-us/windows/win32/winauto/out-of-context-hook-functions)
  and arrive ["in the order in which they are generated"](https://learn.microsoft.com/en-us/windows/win32/winauto/out-of-context-hook-functions).
- **Dropped events.** Microsoft documents none, and says the callback "receives all events". The documented risk
  is the opposite: Windows allocates memory per event, and if the hook is too slow,
  ["USER resources are lowered, eventually resulting in a fault or extremely slow response times"](https://learn.microsoft.com/en-us/windows/win32/winauto/out-of-context-hook-functions).
- **Nothing promises completeness of meaning.** "All events" means all that were raised. No document says every
  move into or out of fullscreen raises one of the events below.
- **Reentrancy.** Calling [`SendMessage`, `GetMessage`, `PeekMessage` and similar](https://learn.microsoft.com/en-us/windows/win32/winauto/guarding-against-reentrancy-in-hook-functions)
  inside the callback can re-enter it. Keep the callback tiny, as [project rule 6](win32-and-rust.md) already says.
- **Scope.** A hook with `idProcess = 0` sees [processes "on the current desktop"](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook).

## Fullscreen without a foreground change

`EVENT_SYSTEM_FOREGROUND` alone misses F11. Chromium enters fullscreen by
[restyling the window and calling `SetWindowPos` with `SWP_NOACTIVATE`](https://github.com/chromium/chromium/blob/main/ui/views/win/fullscreen_handler.cc),
so the foreground window doesn't change. Whether fullscreen video and other browsers take the same path is
**UNCONFIRMED** (P6).

**`EVENT_OBJECT_LOCATIONCHANGE`** fires when
[an object "has changed location, shape, or size"](https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants).

| Point | Finding |
|---|---|
| System sources | Microsoft's page lists ["caret and window objects"](https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants) only. |
| Fires for the cursor? | Not in Microsoft's docs. Third-party sources all say yes: the [`wineventhook` example](https://github.com/OpenByteDev/wineventhook-rs/blob/master/examples/mouse.rs), [Pavel Yosifovich](https://scorpiosoftware.net/2023/09/24/windows-hook-events/), and project reports from [gameframe](https://github.com/ranarn/gameframe/issues/174) and [winshell](https://github.com/dante-sparras/winshell/pull/675). Microsoft defines [`OBJID_CURSOR`](https://learn.microsoft.com/en-us/windows/win32/winauto/object-identifiers) as "not a child of any window". |
| Fires for the caret? | Yes, documented. Winshell reports it on every keystroke, which matters for SEN-02 below. |
| Volume | **UNCONFIRMED.** No source gives a number. Those two projects describe waking on every mouse move and keystroke system-wide. |
| Filter | [Chen's example](https://devblogs.microsoft.com/oldnewthing/20210104-00/?p=104656): `hwnd == target && idObject == OBJID_WINDOW && idChild == CHILDID_SELF`. [komorebi](https://github.com/LGUG2Z/komorebi/blob/master/komorebi/src/windows_callbacks.rs) also returns early unless `id_object == OBJID_WINDOW`. |
| Cutting volume | Filtering in the callback isn't enough, because each event was already copied across and allocated. Scope the hook instead: [Chen's example](https://devblogs.microsoft.com/oldnewthing/20210104-00/?p=104656) passes the target's process and thread IDs. |

Scoping by process is the key lever, but **whether a process-scoped hook still receives cursor events is
UNCONFIRMED** (P1), and the CPU argument rests on it. Scoping to the thread as well is tighter, but
Inference: a resize made from another thread of that process could be missed, so use the process.

**Other candidates:**

| Candidate | Verdict |
|---|---|
| `EVENT_SYSTEM_MOVESIZEEND` | Reports [the end of a move or resize](https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants). [Winshell reports](https://github.com/dante-sparras/winshell/pull/675) it doesn't fire for Win+Up or snapping. Inference: `SetWindowPos`-driven fullscreen doesn't fire it either. Not a substitute. |
| `EVENT_SYSTEM_MINIMIZESTART`, `MINIMIZEEND` | Dropped after P6. Whether restoring a minimized window always raises a foreground event is **UNCONFIRMED**: a [CodeProject question](https://www.codeproject.com/Questions/328990/answer), seen only as a search snippet because the page wouldn't load, says it doesn't. [PowerToys Always On Top](https://github.com/microsoft/PowerToys/blob/main/src/modules/alwaysontop/AlwaysOnTop/AlwaysOnTop.cpp) hooks both. |
| `EVENT_OBJECT_DESTROY` | Dropped after P6. PowerToys comments that ["for some EVENT_OBJECT_DESTROY doesn't work"](https://github.com/microsoft/PowerToys/blob/main/src/modules/alwaysontop/AlwaysOnTop/AlwaysOnTop.cpp), so don't rely on it alone. |
| `EVENT_SYSTEM_DESKTOPSWITCH` | Dropped after P6: the foreground event fired with it. Reports [the active desktop changing](https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants), which lets us re-check after the secure desktop. |
| `ABN_FULLSCREENAPP` | The shell's own decision as an event. An appbar gets it [when a fullscreen app starts or the last one closes](https://learn.microsoft.com/en-us/windows/win32/shell/application-desktop-toolbars), and only removed appbars stop receiving notifications. It needs a registered appbar, and `ABM_NEW` alone, without `ABM_SETPOS`, was enough. **Used**, as a trigger only: see [the measurements](#measured-on-this-machine). |
| `HSHELL_RUDEAPPACTIVATED` | [`RegisterShellHookWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registershellhookwindow) delivers it, but the function is marked "not intended for general use". Avoid. |
| `SHQueryUserNotificationState` | Reports shell state, including [exclusive Direct3D](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-query_user_notification_state). Query only: ["there are no notifications sent when the user starts or stops a full-screen application"](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate). Useful inside the check, not as a trigger. |

## Elevated, exclusive-fullscreen and UWP windows

- **Elevated windows.** UIPI [blocks](https://learn.microsoft.com/en-us/previous-versions/dotnet/articles/bb625963(v=msdn.10)) messages, thread and
  journal hooks, and DLL injection into higher-level processes. Out-of-context WinEvents aren't on that list, but
  I found no statement that they are delivered. The same list says a lower process can't "perform a window handle
  validation" of a higher one, so whether `GetClientRect` works on an elevated window is **UNCONFIRMED** (P4). Design
  for failure: if a geometry call fails, fall back to the shell state.
- **Exclusive fullscreen.** `QUNS_RUNNING_D3D_FULL_SCREEN` [reports it directly](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-query_user_notification_state),
  and [Chromium counts it as fullscreen](https://github.com/chromium/chromium/blob/main/ui/base/fullscreen_win.cc).
  Inference: such a game still owns a foreground window, so the foreground event fires. **UNCONFIRMED** (P6).
- **UWP and WinUI.** Only in-context hooks have [Store app restrictions](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook),
  so out-of-context hooks are unaffected. Whether a UWP app's window belongs to a host process, which would change
  which process to scope to, is **UNCONFIRMED** (P6).

## CPU compared with a 500 ms poll

**No source I found quantifies either side.** What's supportable:

- Out-of-context hooks are ["noticeably slower"](https://learn.microsoft.com/en-us/windows/win32/winauto/out-of-context-hook-functions)
  than in-context ones, per event. Their cost scales with event count, which is why scoping matters.
- Inference: a thread blocked in `GetMessageW` uses no CPU, and the foreground event only fires on a real change.
- Inference: a 500 ms poll is two wake-ups a second. The Overlay's 8 ms thread is about 125, so while the Pet is
  visible, a 500 ms poll is a rounding error against the spike's 0.455 % (RESULTS.md:23).

**So the case for events is not CPU while visible.** It's latency (immediate, against an average 250 ms) and
Hidden: `docs/PRD.md:175` wants zero impact during fullscreen, and with the 8 ms thread stopped, events leave almost
nothing running. A global `LOCATIONCHANGE` hook could cost more than a poll. That's why it must be scoped.

## SEN-02 and security

| Question | Answer |
|---|---|
| Is it a keyboard hook? | No. It's the accessibility event API. The callback receives an event ID, window, object IDs and a time (`<reg>/windows-0.62.2/src/Windows/Win32/UI/Accessibility/mod.rs:21016`), with no key data. Keyboard hooks are `SetWindowsHookEx`. |
| Does it need a DLL? | No. With `WINEVENT_OUTOFCONTEXT`, [`hmodWinEventProc` is NULL](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook) and the callback is ["not mapped into the address space"](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook) of other processes. |
| Antivirus and keylogger optics? | [MITRE's keylogging page](https://attack.mitre.org/techniques/T1056/001/) names `SetWindowsHook` and `GetAsyncKeyState`, not `SetWinEventHook`. [Elastic's keylogger detection article](https://www.elastic.co/security-labs/protecting-your-devices-from-information-theft-keylogger-protection) doesn't mention `SetWinEventHook` or `RegisterHotKey`. [PowerToys](https://github.com/microsoft/PowerToys/blob/main/src/modules/alwaysontop/AlwaysOnTop/AlwaysOnTop.cpp) ships this API. Whether any given product flags it is **UNCONFIRMED**. |

| SEN-02 item | Touched? |
|---|---|
| Keyboard hooks | No. |
| Keystroke counting | **Not by design, but be careful.** Caret `LOCATIONCHANGE` events track typing in the hooked process. Drop them in the first line of the callback, and never count or log unfiltered events. |
| Window titles | No, as long as we never call `GetWindowTextW` and never subscribe to `EVENT_OBJECT_NAMECHANGE`, which exists to report name changes. |
| Screenshots | No. Capture stays in the test binary. |

**Decision for the owner.** SEN-01 says the Pet may read cursor position and idle time "only", and SEN-02 defers
"active-app awareness". WIN-07 can't work without reading the foreground window's geometry, class and process ID.
Neither is a title or an app name, and this design never turns a process ID into a name, but the PRD wording doesn't
strictly allow it. I'd add a one-line exception to SEN-01.

## Which rectangle matches the shell

[Chen's rule](https://devblogs.microsoft.com/oldnewthing/20250522-00/?p=111211): "If the client area covers the
entire screen, then it is a fullscreen window", and "the fullscreen calculations look only at the client area". He
says this works "fairly well most of the time", and the full algorithm isn't published.

| Method | Measures | Matches the shell? |
|---|---|---|
| **`GetClientRect` + `ClientToScreen`** | Client area. [`GetClientRect`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect) is relative to the window, so convert both corners with `ClientToScreen`. Inference: that keeps every value in screen coordinates, which matters if the window is DPI-unaware. | **Yes. Use this.** |
| `GetWindowRect` | Whole window, which [may include invisible resize borders](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect). | No. Inference: a maximized window's rect can pass despite having a title bar, which is why [Chromium](https://github.com/chromium/chromium/blob/main/ui/base/fullscreen_win.cc) adds style checks. |
| `DWMWA_EXTENDED_FRAME_BOUNDS` | [Visible frame, not adjusted for DPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect), including the title bar. | No. |

Get the monitor rect from `MonitorFromWindow` then `GetMonitorInfoW`, field `rcMonitor`, not `rcWork`. Compare by
containment ("covers"), not equality. Re-read it every time, because a game can change the display mode.

**Two overrides to honour:**

- A window can declare itself [never fullscreen](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist2-markfullscreenwindow)
  with the `NonRudeHWND` window property. Reading it with `GetPropW` matches the shell. Whether `GetPropW` works
  across processes is **UNCONFIRMED**.
- Set `NonRudeHWND` on our own Overlay [before showing it](https://devblogs.microsoft.com/oldnewthing/20250522-00/?p=111211),
  because a work-area overlay can be screen-sized when the taskbar auto-hides. Chen's example is exactly this kind of
  screen-covering effect window. The proposal's `window_styles.rs` already plans this.

## `windows` 0.62 features and signatures

Add `Win32_UI_Accessibility` and `Win32_UI_Shell` to the spike's list. `Win32_Foundation`, `Win32_Graphics_Gdi` and
`Win32_UI_WindowsAndMessaging` are already [in it](../../spikes/bevy-overlay/Cargo.toml). `Win32_Graphics_Dwm` is only
needed if you read cloaking or extended frame bounds.

| Item | Module | Signature | Source (`<reg>/windows-0.62.2/src/Windows/Win32/`) |
|---|---|---|---|
| `SetWinEventHook` | `UI::Accessibility` | `(eventmin: u32, eventmax: u32, hmodwineventproc: Option<HMODULE>, pfnwineventproc: WINEVENTPROC, idprocess: u32, idthread: u32, dwflags: u32) -> HWINEVENTHOOK` | `UI/Accessibility/mod.rs:222` |
| `UnhookWinEvent` | `UI::Accessibility` | `(hwineventhook: HWINEVENTHOOK) -> BOOL` | `:675` |
| `WINEVENTPROC` | `UI::Accessibility` | `Option<unsafe extern "system" fn(HWINEVENTHOOK, u32, HWND, i32, i32, u32, u32)>` | `:21016` |
| `GetForegroundWindow` | `UI::WindowsAndMessaging` | `() -> HWND` | `UI/WindowsAndMessaging/mod.rs:866` |
| `GetWindowThreadProcessId` | same | `(hwnd: HWND, lpdwprocessid: Option<*mut u32>) -> u32` | `:1184` |
| `GetClientRect` | same | `(hwnd: HWND, lprect: *mut RECT) -> Result<()>` | `:805` |
| `GetShellWindow` | same | `() -> HWND` | `:1071` |
| `GetClassNameW` | same | `(hwnd: HWND, lpclassname: &mut [u16]) -> i32` | `:795` |
| `GetPropW` | same | `(hwnd: HWND, lpstring: P1) -> HANDLE` | `:1038` |
| `IsIconic` | same | `(hwnd: HWND) -> BOOL` | `:1320` |
| `GetMessageW` | same | `(lpmsg: *mut MSG, hwnd: Option<HWND>, min: u32, max: u32) -> BOOL` | `:997` |
| `SetTimer` | same | `(hwnd: Option<HWND>, nidevent: usize, uelapse: u32, lptimerfunc: TIMERPROC) -> usize` | `:2238` |
| `KillTimer` | same | `(hwnd: Option<HWND>, uidevent: usize) -> Result<()>` | `:1365` |
| `ClientToScreen` | `Graphics::Gdi` | `(hwnd: HWND, lppoint: *mut POINT) -> BOOL` | `Graphics/Gdi/mod.rs:120` |
| `MonitorFromWindow` | same | `(hwnd: HWND, dwflags: MONITOR_FROM_FLAGS) -> HMONITOR` | `:1484` |
| `GetMonitorInfoW` | same | `(hmonitor: HMONITOR, lpmi: *mut MONITORINFO) -> BOOL`; set `cbSize` first | `:1102`, `:5835` |
| `SHQueryUserNotificationState` | `UI::Shell` | `() -> Result<QUERY_USER_NOTIFICATION_STATE>` | `UI/Shell/mod.rs:3744` |

Constants are in `UI::WindowsAndMessaging`: `EVENT_SYSTEM_FOREGROUND` (3),
`EVENT_SYSTEM_MINIMIZESTART` (22), `EVENT_SYSTEM_MINIMIZEEND` (23), `EVENT_SYSTEM_DESKTOPSWITCH` (32),
`EVENT_OBJECT_DESTROY` (32769), `EVENT_OBJECT_LOCATIONCHANGE` (32779), `WINEVENT_OUTOFCONTEXT` (0),
`WINEVENT_SKIPOWNPROCESS` (2), `OBJID_WINDOW` (a typed value, compare `.0`), `CHILDID_SELF` (0). Note the callback's
`idchild` is `i32` while `CHILDID_SELF` is `u32`. [`wineventhook`](https://github.com/OpenByteDev/wineventhook-rs)
0.11.0 wraps this, but direct calls fit the project's thin-wrapper rule.

## Known failure modes

| Failure | Cause | Handling |
|---|---|---|
| False Auto-hide when clicking the desktop | The desktop covers the monitor and becomes foreground | Exclude [`GetShellWindow()`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getshellwindow) and the `Progman` and `WorkerW` classes. Those class names are undocumented, so test them (P6). A class name is not a title, so SEN-02 is fine. |
| Pet stuck Hidden | Exit event missed | The 2 s re-check. This is the worst failure, because the Pet silently disappears. Measured: a fullscreen window hidden rather than destroyed does report its exit. |
| Foreground is NULL | [Happens during activation changes](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow). Measured: about 120 ms when a fullscreen window hides, and seconds around the lock screen. | Keep the previous state, wait for the next trigger. |
| Foreground event arrives before the final size | The window is activated, then resized | Debounce. |
| Slow hook hurts the system | [Documented](https://learn.microsoft.com/en-us/windows/win32/winauto/out-of-context-hook-functions) | Hook only the foreground event, keep the callback trivial. |
| Pet appears during Alt-Tab out of a game | Measured: the switcher takes the foreground, an exclusive game minimizes, and the shell reports the exit | Accepted by the owner: the game really has left. The return delay is one value, so it can grow. |
| Maximized app with no title bar and an auto-hidden taskbar | Client area may cover the monitor | May hide wrongly. The shell has the same input, so it may agree. **UNCONFIRMED** (P6). |
| DPI-unaware fullscreen app under display scaling | Client rect may be reported in scaled units | Measured: not a problem. A per-monitor-aware reader gets physical pixels. |
| Elevated foreground window | Geometry calls may fail | Measured: they don't fail on Task Manager, so there's no fallback. |
| Lock screen | Its own fullscreen window | Measured: it counts as fullscreen, so the Pet is Hidden while locked, and the foreground event on unlock brings it back. |
| Explorer restarts | The new shell has no appbars | Register again on `TaskbarCreated`. Until then the foreground hook and re-check still work. |

## Recommended design

The owner chose this on 2026-11-01 from the [measurements](#measured-on-this-machine). It's smaller than the first
version of this section, which had five hooks and swapped two of them on every foreground change. It fits the
[proposal's](../proposals/item-2-architecture.md) modules: `windows/foreground.rs` on `shell_thread.rs`, with the
decision in `pet-core/src/fullscreen.rs`.

**Three triggers, which all do the same thing:**

1. `ABN_FULLSCREENAPP`, from an appbar registered with `ABM_NEW` alone. This is what catches fullscreen with no
   foreground change.
2. `EVENT_SYSTEM_FOREGROUND`, with `WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS` and process 0. The shell's message
   can say fullscreen before the fullscreen window is in front, and then has nothing more to say when it arrives.
3. A 2 s timer. Two foreground changes in the probes reached only this. It also bounds the delay if a future Windows
   stops sending the first one.

`EVENT_OBJECT_LOCATIONCHANGE`, `EVENT_OBJECT_DESTROY`, the minimize events and `EVENT_SYSTEM_DESKTOPSWITCH` are gone.
In every real fullscreen change, the appbar or the foreground hook fired as well. Without `LOCATIONCHANGE` there are no
caret events to drop, so nothing near typing ever reaches this process.

**A trigger only restarts a timer.** Win32 has no one-shot timer, so the `WM_TIMER` handler must call
[`KillTimer`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-killtimer). The wait is 100 ms before
hiding. Coming back has its own value, also 100 ms for now, which settings replace in roadmap item 6.

**When the timer fires,** build a plain snapshot and call a pure function in `pet-core`:

```text
fullscreen = shell_state is RUNNING_D3D_FULL_SCREEN
          or (foreground exists
              and not the desktop
              and not marked NonRudeHWND
              and client area covers the Home monitor)
```

If there's no foreground window, or its client area can't be read, the answer stays as it was. `IsIconic` isn't needed,
because a minimized window's client area is empty. Class names and process IDs aren't needed either. The one thing read
beyond geometry is `NonRudeHWND`, a flag a window sets to say it's never fullscreen. It's needed because the desktop and
the Alt-Tab window both set it, and SEN-01's exception names it.

**One hidden top-level window owns all of it.** A message-only window hears the shell's message but, as a Windows rule,
no broadcasts, and a restarted Explorer has forgotten every appbar. The
[`TaskbarCreated`](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar) broadcast is the cue to register again.

**Tested against the real app, 2026-11-01.** Broken on purpose, the new tests failed for the right reasons. With no watcher,
all three behaviour tests failed. With only the foreground hook left, by removing the shell's message and the first check and
slowing the re-check to 60 s, a fullscreen window coming to the front still hid the Pet, and closing it brought the Pet
back. Growing a window in front did not hide it, and launching behind one showed it. So each trigger does the job it's
here for.

During an Auto-hide the debug build used 0.26% of one core, against a limit of 0.5%, with GPU counter 0, over 12 s
holding six re-checks.

**Multiple monitors are out of scope,** because this machine can't test them. The rule compares against one rect. So a
borderless fullscreen app on another monitor leaves the Pet shown, and an exclusive game there would hide it, because
the shell state isn't tied to a monitor.

# Measured on this machine

**Setup, 2026-11-01:** Windows 11 build 26200, one monitor at 3840×2160 and 150% scaling, work area 3840×2088. Throwaway
programs outside the repo, with the owner working through each app by hand. They read geometry, class name and process
ID only, dropped caret events first, and never counted them. All the Pet figures are for a debug build.

**Does the Pet keep rendering behind a fullscreen game? Yes.** Read from outside the app:

| While | CPU, share of one core | GPU |
|---|---|---|
| Nothing fullscreen, three readings | 16.6% to 20.9% | 2.0% to 10.2% |
| Behind a fullscreen game (`QUNS_RUNNING_D3D_FULL_SCREEN`), 30 s | 26.8% | 4.15% |

So without Auto-hide the Pet misses the "zero impact" budget row in full, even though the game covers it.

**`ABN_FULLSCREENAPP` works (P5).** `ABM_NEW` without `ABM_SETPOS` returned success and left the work area unchanged, on a
hidden top-level window and on a message-only window. Both then got the message for every fullscreen entry and exit
below, within 70 ms of the change for the probe's own windows. It works with no foreground change: Chrome F11 and a
window resized to fill the monitor while already in front. In 11 minutes the appbars received no other notification.

**It's a trigger, not an answer.** For 10 s during the session it reported fullscreen while the foreground window wasn't,
and `SHQueryUserNotificationState` said `QUNS_BUSY` for the same 10 s. Both describe the shell's view, not "the
foreground window". `QUNS_BUSY` also lingered briefly during Alt-Tab, so it stays out of the rule.

**What each case looked like (P6).** "Covers" means the client area covers the monitor.

| Case | Foreground class | Covers | `QUNS_*` | Triggers that fired |
|---|---|---|---|---|
| Chrome F11, in and out | `Chrome_WidgetWin_1` | Yes, then no | `BUSY`, then `ACCEPTS_NOTIFICATIONS` | Appbar and `LOCATIONCHANGE`, no foreground event |
| Fullscreen video in Chrome | `Chrome_WidgetWin_1` | Yes | `BUSY` | Appbar and `LOCATIONCHANGE` |
| Calculator with Win+Shift+Enter | `ApplicationFrameWindow`, in a host process | Yes | `APP` | Appbar and `LOCATIONCHANGE` |
| Borderless monitor-sized window, including one DPI-unaware | Probe's own | Yes, in physical pixels at 150% | `BUSY` | Appbar |
| Unity game | `UnityWndClass` | Yes | `RUNNING_D3D_FULL_SCREEN` | Appbar |
| SDL game | `SDL_app` | Yes | `RUNNING_D3D_FULL_SCREEN` | Appbar and foreground |
| Alt-Tab out of a game | `XamlExplorerHostIslandWindow` | No | `BUSY`, then `ACCEPTS_NOTIFICATIONS` | Appbar and foreground, and the game minimized |
| Clicking the desktop | `Progman`, `NonRudeHWND` set | Yes, so it must be excluded | `ACCEPTS_NOTIFICATIONS` | **Only the 2 s re-check** |
| Maximized Task Manager | `TaskManagerWindow` | No: bottom edge stops at the taskbar | `ACCEPTS_NOTIFICATIONS` | `LOCATIONCHANGE` |
| Lock screen | `Windows.UI.Core.CoreWindow` | Yes | `NOT_PRESENT` | Desktop switch and foreground |

- **Geometry alone found every fullscreen case,** including both games, so nothing here needed the shell state. A game
  that changes the display mode wasn't tried.
- **The 2 s re-check earned its place.** Two foreground changes, to the desktop and to a windowed `SDL_app`, raised no
  foreground event at all. Neither was a fullscreen change, but nothing else saw them.
- **A hidden fullscreen window** went through a NULL foreground for about 120 ms, and the appbar reported the exit.
- **`GetPropW` works across processes:** it read `NonRudeHWND` on `Progman` and on the Alt-Tab window.
- **The Windows hooks skip our own process,** so the probe's own windows only tested the appbar. Every hook row above comes
  from a real app. A resize from a second process was caught by the appbar, so whether a scoped hook sees it is still
  unknown.

**Hook volume (P1).** Passive counts with the owner working normally:

| Hook on `LOCATIONCHANGE` | Cursor events | CPU of the program holding the hook |
|---|---|---|
| All processes, 50 s | 22.2 a second | 0.156% of one core |
| Foreground process, 50 s | 14.8 a second | Too small to read |
| Scoped to the game, 30 s | 32.3 a second | Too small to read |

Scoping to a process doesn't remove cursor events. Three hooks held at once over the same 45 s counted 1448 for all
processes, 932 for the process under the cursor, and 0 for the Pet's process, so a scoped hook gets them only while the
cursor is over that process. Even the unscoped cost is small. The case against this hook is about privacy and
complexity, not CPU.

**Elevated windows (P4).** With Task Manager in front, foreground and `LOCATIONCHANGE` events arrived, and `GetClientRect`,
`ClientToScreen`, `GetWindowRect` and `GetPropW` all succeeded. The account is an administrator with UAC on, and the probe
wasn't elevated. Task Manager rises without a prompt on such an account, but its level wasn't read directly, so this is
strong evidence, not proof.

# Unconfirmed items

**Probes.** Each is a short throwaway program.

| # | Probe | Result or why it matters |
|---|---|---|
| P1 | Hook `EVENT_OBJECT_LOCATIONCHANGE` globally and scoped to one process. Count events by `idObject` while moving the mouse and typing. | **Done 2026-11-01.** [Hook volume](#measured-on-this-machine). |
| P2 | Start the spike, walk the UIA tree in raw and control views, look for the Overlay. Also list tray icons. | Open. Whether UIA can see the Overlay at all. |
| P3 | **Done 2026-11-01:** injected keys trigger `RegisterHotKey`. See [Hotkey delivery](#check-by-check). | The hotkey test can use real delivery. |
| P4 | With an elevated window such as Task Manager in front: do foreground events arrive, and do `GetClientRect` and `GetPropW` succeed? | **Done 2026-11-01.** All yes, so no elevated fallback is needed. |
| P5 | Create a hidden window, send `ABM_NEW` without `ABM_SETPOS`. Check the work area is unchanged, then toggle F11 and watch for `ABN_FULLSCREENAPP`. | **Done 2026-11-01.** It works, as a trigger only. |
| P6 | Log the snapshot for: Chrome F11, fullscreen video, Edge, a borderless game, an exclusive game, a UWP app, a DPI-unaware borderless fullscreen app at the machine's scaling, a maximized titleless app with auto-hide taskbar, clicking the desktop, Alt-Tab in a game, and each `QUNS_*` value. | **Mostly done 2026-11-01.** Not run: Edge, a maximized titleless app with an auto-hide taskbar, a game that changes the display mode. `QUIET_TIME` and `PRESENTATION_MODE` never appeared. |
| P7 | A workflow on `windows-latest` that starts the Overlay, captures it and clicks through it. | Open. Whether real-exe tests can run on hosted CI. |

**Other open items:**

- `bevy_remote`'s memory and CPU cost: measure it with the spike's metrics, with and without the plugin.
- `xcap`'s `SRCCOPY` capture of the Overlay, only if you decide to use `xcap`.
- Whether `dxcap -forcetdr` needs elevation.
- Whether `insta` has any tolerance for image snapshots.
- Whether any antivirus product flags `SetWinEventHook`.
- Whether this machine supports S3 sleep or Modern Standby: `powercfg /a` answers it.
