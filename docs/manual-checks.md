# Manual checks

Only what can't be automated belongs here. Everything else is a test run by `cargo e2e`.

| Check | How | Expected |
|---|---|---|
| Auto-hide taskbar | Turn on "Automatically hide the taskbar", run the app, and move the cursor to the bottom edge. | The taskbar slides up over everything as usual, and stays usable while the app runs. |
| Lock and unlock | Press Win+L, then sign back in. | The Pet is still there and clicks still pass through. |
| Sleep and resume | Put the machine to sleep and wake it. | The Pet is still there and clicks still pass through. |
| Art | Look at the Pet at each integer scale. | Crisp pixels, no blur or shimmer. |
