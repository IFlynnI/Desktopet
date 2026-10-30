# Spike run summary

- Duration: 1.5 min
- CPU (whole machine, 32 logical cores, after 20s warm-up): avg 0.445% / max 0.532%
- Avg fps: 30.0
- Max working set: 225.4 MB
- Max private bytes (commit): 358.1 MB
- Max private working set (Task Manager "Memory"): 152.0 MB
- Hit-test style toggles: 1
- Style heals (someone else reset our ex-style): 1
- Transparency checks: 1 ok / 0 failed

## Notes

- config: backend=dx12 present=visual alpha=premultiplied fps_cap=30 scale=4x poll=8ms msaa=1 layered_attrs=false window=512x512 at (0,1576)
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- ex-style: TOOLWINDOW=true APPWINDOW=false NOACTIVATE=true TOPMOST=true LAYERED=true (TOOLWINDOW && !APPWINDOW ⇒ hidden from taskbar and Alt-Tab)
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
