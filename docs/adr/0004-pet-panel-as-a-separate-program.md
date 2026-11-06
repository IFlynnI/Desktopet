# The Pet panel is a separate Tauri program that edits files, reads a live pipe and sends a few commands

The Pet panel is the app's one ordinary window. It holds the Settings, the Mind inspector, balance tuning and, after
the MVP, the visual Mind editor. It is its own program, `pet-panel`, built with Tauri 2 on WebView2. Its UI is React
with React Flow for node graphs, written in TypeScript and managed with `pnpm`. The tray and the right-click menu start
it. When the owner changes something, the panel writes the file in `%LOCALAPPDATA%\DesktopPet\config\`, and the Pet's
one hot reload system applies it. So an edit made in the panel and an edit made by hand take the same path, with the
same validation and the same errors. Data that changes all the time, such as drives, scores, reasons and memories,
flows from the Pet to the panel, over a named pipe that only the owner's account can open. It runs at about 10
updates a second, and only while the panel is open. The same pipe carries a small, closed set of commands the other
way, for what the Pet itself owns: evolve, erase a Link, reset learning, reset the Rhythm, clear the decision log, and
import a save. The message types for both directions live in `pet-protocol`, a crate with no workspace dependencies,
so the panel's Rust shell doesn't pull in the Pet's OS code.

The reason is that it must cost nothing when closed. We measured the options against a copy of the real Overlay
([dev-panel.md](../research/dev-panel.md)):

| Option | Effect on the Pet's RAM | Looks and creative freedom |
|---|---|---|
| A second window inside the Pet, with `bevy_egui` 0.43.1 | The first open kept 18–30 MB for good. Later opens crept up by 2–4 MB each: 124.5 → 128.2 → 130.8 → 132.9 MB over four cycles | egui: reachable, but nearly everything is painted by hand |
| `bevy_ui` with feathers | It kept 42 MB after closing, and added 24.5 MB to the exe | No chart or node-graph widget, and feathers isn't meant for end-user UI |
| A separate egui program | Back to about 96 MB once it exits. About 87 MB while it's open | As for egui above |
| A separate WebView2 program | The same as the separate egui program | The highest ceiling: CSS animation, browser devtools, live reload, and built-in accessibility and IME |

Looks decided between the two separate programs. The owner wants room to be creative with UX and UI. The same
mockup was built in egui, Slint, bevy_ui and HTML ([panel-visuals](../research/panel-visuals/index.html)), and the
HTML one reached the look most easily. Tauri adds packaging and a dev server with live reload on top of WebView2.

## Consequences

- With the panel open, the total is about 184 MB and 1.4–2.0 % CPU, which is over the < 150 MB and < 1 % budget. §6.2
  therefore applies that budget while the panel is closed. While it's open there's a loose guideline of about 200 MB
  and 2 %, because the owner is looking at the panel rather than working.
- The project gains a second language, TypeScript, and a web build step through `pnpm`.
- What the owner configures (settings, balance, Mind overrides) goes through a file the Pet already validates.
  What the Pet owns goes through a command: Progress, learned Links, the Rhythm, the decision log and the save are
  written by the Pet, and it rewrites the save every 2 s, so a file edit from the panel could be lost or could race
  the Pet's own write. The Pet carries each command out inside its tick and replies with the result. The command list
  is closed, so a new one is a deliberate change to `pet-protocol`.
- Export stays a plain copy of the save file. Import is a command, so the Pet checks the file, backs up its save and
  swaps it in itself.
- Dev-only parts live on a Developer tab that the owner switches on in Settings.
- The visual Mind editor (Track 1) lives in this program and writes the per-Character Mind overrides.
