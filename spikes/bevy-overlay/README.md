# Bevy overlay platform spike (THROWAWAY)

Answers the go/no-go question in [docs/PRD.md §7.1](../../docs/PRD.md): can Bevy 0.20 give us a
transparent, always-on-top, per-pixel click-through overlay on Windows 11 + NVIDIA, within budget,
**without** any driver-setting changes?

Not production code. The only module worth carrying forward is `src/hit_mask.rs` (pure, unit-tested).

## What it does

- Transparent, borderless, topmost window covering the primary monitor's work area.
- DX12 with the DXGI *Visual* (DirectComposition) swapchain, selected in-process via
  `WGPU_DX12_PRESENTATION_SYSTEM=Visual`, `CompositeAlphaMode::PreMultiplied`, `ClearColor(NONE)`.
- A procedural placeholder pet (with holes) that walks along the taskbar; drag it and drop it to see gravity.
- `hit-test` thread polls `GetCursorPos` every 8 ms — independent of the 30 fps render cap — and
  flips `WS_EX_TRANSPARENT` from the pet's alpha mask. It also keeps `WS_EX_TOOLWINDOW`
  (no taskbar / Alt-Tab), `WS_EX_NOACTIVATE` (clicking the pet doesn't steal focus) and `WS_EX_TOPMOST`,
  re-applying them if winit resets them.
- `metrics` thread logs CPU / memory / fps every 10 s to `metrics.csv`, runs a hide-vs-show screen
  comparison every 5 min (catches the "black window" bug), and writes `summary.md` at the end.
- Cursor self-test (12 s after start): sweeps the real cursor across the pet and checks with
  `WindowFromPoint` that only opaque pixels hit our window. **It moves your mouse for ~3 s.**

## Run

```bash
cargo test --release
```

```bash
cargo build --release
```

```bash
./target/release/bevy-overlay.exe --minutes 60 --out results/my-run
```

Flags: `--minutes N` · `--fps N` (cap, default 30) · `--scale N` (default 4) · `--backend dx12|vulkan` ·
`--present visual|hwnd` · `--alpha premultiplied|postmultiplied|auto` · `--msaa 1|2|4` (default 1) ·
`--mem-hint usage|performance` (default usage) · `--win WxH` (small window, bottom-left) ·
`--poll-ms N` (default 8) · `--no-selftest` · `--layered-attrs`.

## Results

See [RESULTS.md](RESULTS.md).
