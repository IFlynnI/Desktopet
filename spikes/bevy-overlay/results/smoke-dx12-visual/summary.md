# Spike run summary

- Duration: 2.0 min
- CPU (whole machine, 32 logical cores, after 20s warm-up): avg 0.463% / max 0.630%
- Avg fps: 31.1
- Max working set: 246.9 MB
- Max private bytes: 623.7 MB
- Hit-test style toggles: 13
- Style heals (someone else reset our ex-style): 1
- Transparency checks: 1 ok / 0 failed

## Notes

- config: backend=dx12 present=visual alpha=premultiplied fps_cap=30 scale=4x poll=8ms layered_attrs=false window=3840x2088 at (0,0)
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- ex-style: TOOLWINDOW=true APPWINDOW=false NOACTIVATE=true TOPMOST=true LAYERED=true (TOOLWINDOW && !APPWINDOW ⇒ hidden from taskbar and Alt-Tab)
- cursor self-test: 224 points across sprite rows 5/8/12/16 (incl. holes) at 4x, settle 30ms → 0 misrouted
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
