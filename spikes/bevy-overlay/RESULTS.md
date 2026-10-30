# Spike results — Bevy 0.20.1 overlay on Windows 11 + RTX 2080 Ti

**Date:** 2026-10-30 · **Machine:** RTX 2080 Ti (driver 32.0.16.1088), 3840×2160, 32 logical cores ·
**Driver settings:** untouched defaults.

## Verdict

**Criteria 1–3 pass. Criterion 4 passes on CPU, fps and stability but missed the original memory
budget by 1.3 MB (101.3 MB private working set vs < 100 MB).**

**Decision (2026-10-30):** the owner raised the budget to **< 150 MB** private working set and adopted
Bevy (PRD §6.2, §7.1).

Note: memory steps from ~92 MB to ~100 MB at ~20 s in every run with the cursor self-test, i.e. once
the pet is actually interacted with. Treat ~100 MB as the real baseline; any lever needs to recover
≥ 5 MB to give real headroom.

| # | Criterion (PRD §7.1) | Result |
|---|---|---|
| 1 | Transparent window, default driver settings | ✅ DX12 + DXGI Visual swapchain + `PreMultiplied` + `ClearColor(NONE)`, selected in-process. 13/13 hide-vs-show checks identical over 60 min; no black window. The NVIDIA black-window bug ([bevy#7544](https://github.com/bevyengine/bevy/issues/7544)) did not reproduce on DX12 (nor in a short Vulkan run). |
| 2 | Per-pixel click-through, no misrouted edge clicks | ✅ Cursor self-test: 224 points across 4 sprite rows incl. holes → 0 misrouted. Owner confirmed by hand: hitbox correct, drag & drop works, clicks beside/between the legs reach the app below. |
| 3 | Hidden from taskbar and Alt-Tab | ✅ `WS_EX_TOOLWINDOW`, no `WS_EX_APPWINDOW`; owner confirmed: not in Alt-Tab, not on taskbar. No focus steal on click is set via `WS_EX_NOACTIVATE` but not yet confirmed by hand. |
| 4a | CPU < 1 % for 1 h at 30 fps | ✅ avg **0.455 %**, max 0.742 % of the machine (≈ 15 % of one core). |
| 4b | fps | ✅ avg 30.3; brief 42–48 fps bursts while the cursor interacts with the pet (window events wake the reactive loop early). |
| 4c | RAM < ~100 MB | ⚠️ **101.3 MB** private working set (Task Manager "Memory"); 166.8 MB working set; 283.9 MB commit. |
| 4d | No black-window failures over 1 h | ✅ none. |

## Memory investigation

| Run (2 min unless noted) | Private working set |
|---|---|
| Defaults (MSAA 4×, `MemoryHints::Performance`) | ~250 MB WS / 624 MB commit (private WS not yet measured) |
| MSAA off | 151.9 MB |
| MSAA off, 512×512 window instead of full work area | 152.0 MB → **cost is independent of window size** |
| MSAA off + `MemoryHints::MemoryUsage` (no self-test) | 91.9 MB |
| Same, Vulkan backend | 231.6 MB → DX12 is the path |
| Same + self-test, 8 min (interrupted by sleep) | plateau 102.8 MB |
| **Lean Bevy features, 60 min (final)** | **88.8 → 100.1 MB in 10 min → 101.3 MB at 60 min** |

- The ~100 MB is a fixed cost of Bevy + wgpu + the NVIDIA DX12 user-mode driver, not the 4K canvas.
- Dropping post-processing/gizmo features saved only ~1.5 MB.
- **Slow creep:** +1.2 MB between minute 10 and minute 60 (~25 KB/min). Could be allocator
  fragmentation or a small leak. Needs a multi-hour run in the real app before calling it either way.

### Levers not yet tried
- Single-threaded Bevy (drop `multi_threaded`: fewer task-pool threads and their allocations).
- Disable `LogPlugin` / `bevy_log`.
- `MemoryHints::Manual` with smaller suballocation block sizes.
- Don't render while nothing changes (reactive redraw only on animation frames — also lowers CPU).

## Findings to carry into the real app

1. **Required render config:** DX12 backend, `WGPU_DX12_PRESENTATION_SYSTEM=Visual` set in-process
   before app start, `CompositeAlphaMode::PreMultiplied`, `ClearColor(Color::NONE)`, `Msaa::Off`,
   `MemoryHints::MemoryUsage`, `PipelinedRenderingPlugin` disabled.
2. **Frame cap:** `WinitSettings` with `UpdateMode::reactive_low_power(1/30 s)` holds 30 fps.
3. **Click-through:** let winit start with `hit_test = false`, then drive `WS_EX_TRANSPARENT` from a
   separate 8 ms polling thread against the sprite's alpha mask — decoupled from the render cap
   (REN-05). Re-assert `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST` each poll; winit
   reset them once at startup ("style heals: 1").
4. **DPI:** call `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` before Bevy starts, and use
   `scale_factor_override(1.0)` so window units are physical pixels.
5. A full-work-area overlay costs the same memory as a small window, so the simpler full-screen
   overlay design (needed later for encounters) is fine.
6. `src/hit_mask.rs` (pure, 6 Given-When-Then tests) can move into the domain core as-is.

## Caveats

- One machine, one GPU vendor. AMD/Intel GPUs and multi-monitor setups are untested.
- The transparency check compares the overlay hidden vs shown. When the content underneath is itself
  black it can't tell "transparent" from "black window"; 8 of the 14 checks (13 periodic + startup)
  had mostly non-black content underneath and were conclusive.
- Sleep/resume was not deliberately tested (an earlier run was interrupted by the machine sleeping).
- Screenshots in `results/*/shot_*.png` capture your desktop; they are git-ignored.
