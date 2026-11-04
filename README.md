# Desktop Pet

A desktop companion that lives on top of your other Windows apps. See the [PRD](docs/PRD.md) for what it is and
where it's going, and the [glossary](GLOSSARY.md) for the words used here.

## Run it

The Pet comes from a Pack, a folder of art and data in `%LOCALAPPDATA%\DesktopPet\packs`. The app uses the first one
by folder name. Copy in the placeholder Pack once:

```bash
mkdir -p "$LOCALAPPDATA/DesktopPet/packs" && cp -r content/placeholder "$LOCALAPPDATA/DesktopPet/packs/"
cargo run -p pet-app
```

To use another folder of Packs, set `DESKTOP_PET_PACKS` to it. With no usable Pack there's no Pet, the reasons are in
`%LOCALAPPDATA%\DesktopPet\desktop-pet.log`, and the tray icon still works so you can quit.

Edit the Pack while the app runs, and the Pet changes about a quarter of a second after each save. An edit with
mistakes changes nothing, and the log lists each one.

To check a Pack the way the app does, listing every error:

```bash
cargo run -p pet-content --bin pet-pack -- validate content/placeholder
```

Press Ctrl+Alt+P or left-click the tray icon to hide or show the Pet. To stop it, right-click the tray icon and
choose Quit.

Ctrl+C in that terminal also shuts it down cleanly, but reports exit code 130, the usual code for being stopped that
way. A release build has no terminal to press it in.

## Test it

Fast tests show no window and touch neither the mouse nor the keyboard. The render tests among them draw the Pet
into memory on the GPU and check every pixel:

```bash
cargo nextest run
```

The next two run the real app, so they're for you to start, never something to leave running in the background.
Only one Pet can run at a time, so quit yours first.

Quiet tests show the Pet, its tray icon and the Overlay, but send no keys or mouse input and read nothing off the
screen, so you can keep working:

```bash
cargo e2e-quiet
```

The full set adds tests that type the Ctrl+Alt+P hotkey, take over your mouse, read the screen and open magenta
windows, some filling the screen, so keep your hands off it. It stops with a clear message if the mouse moves:

```bash
cargo e2e
```

A plain `cargo test` runs the fast tests only. Install the runner once with `cargo install cargo-nextest --locked`.

Anything that can't be automated is in [docs/manual-checks.md](docs/manual-checks.md).
