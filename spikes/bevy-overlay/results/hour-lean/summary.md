# Spike run summary

- Duration: 60.1 min
- CPU (whole machine, 32 logical cores, after 20s warm-up): avg 0.455% / max 0.742%
- Avg fps: 30.3
- Max working set: 166.8 MB
- Max private bytes (commit): 283.9 MB
- Max private working set (Task Manager "Memory"): 101.3 MB
- Hit-test style toggles: 147
- Style heals (someone else reset our ex-style): 1
- Transparency checks: 13 ok / 0 failed

## Notes

- config: backend=dx12 present=visual alpha=premultiplied fps_cap=30 scale=4x poll=8ms msaa=1 mem_hint=usage layered_attrs=false window=3840x2088 at (0,0)
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- ex-style: TOOLWINDOW=true APPWINDOW=false NOACTIVATE=true TOPMOST=true LAYERED=true (TOOLWINDOW && !APPWINDOW ⇒ hidden from taskbar and Alt-Tab)
- cursor self-test: 224 points across sprite rows 5/8/12/16 (incl. holes) at 4x, settle 30ms → 0 misrouted
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.95 / shown 0.95 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.17 / shown 0.17 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.27 / shown 0.27 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 1.00 / shown 1.00 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 1.00 / shown 1.00 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 1.00 / shown 1.00 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- transparency check @ patch (100,1928): similarity 1.000, black hidden 0.00 / shown 0.00 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.92 / shown 0.92 → OK
- transparency check @ patch (3540,1928): similarity 1.000, black hidden 0.92 / shown 0.92 → OK
