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

Edit the Pack while the app runs, and the Pet changes about a quarter of a second after each save. It starts its
Form's default Action again from the first Frame. A Pet in its starting Form moves to the new one if you change
`starting_form`; in any other Form it keeps that Form while the Pack still has it. The log says which Form the Pet is
in and which Action it's playing. An edit with mistakes changes nothing, and the log lists each one.

To check a Pack the way the app does, listing every error:

```bash
cargo run -p pet-content --bin pet-pack -- validate content/placeholder
```

Grab the Pet with the left button to pick it up. It hangs from where you grabbed it, swings as you move, and keeps
its speed when you let go, so a flick throws it. It bounces off the edges of the work area and lands on the taskbar.

The placeholder has no art for being held, falling or landing. To see those, use Brim, the first real character,
whose egg has all three. The app picks the first Pack by folder name, and `firstborn` comes before `placeholder`:

```bash
cp -r content/firstborn "$LOCALAPPDATA/DesktopPet/packs/"
```

The feel numbers and walls live in `%LOCALAPPDATA%\DesktopPet\config`. Without those files the Pet uses the defaults
and there are no walls. For example, on a 1920 x 1080 screen, a bouncy wall 400 pixels left of the right edge, a
slippery shelf to land on in the middle, and gravity a little lower:

```bash
mkdir -p "$LOCALAPPDATA/DesktopPet/config"
printf '[[obstacle]]\nrect = { x = 1480, y = 500, width = 40, height = 580 }\nmaterial = { friction = 0.4, bounciness = 0.7 }\n\n[[obstacle]]\nrect = { x = 800, y = 700, width = 320, height = 20 }\nmaterial = { friction = 0.05, bounciness = 0.0 }\n' > "$LOCALAPPDATA/DesktopPet/config/obstacles.toml"
printf '[physics]\ngravity = 2500\n' > "$LOCALAPPDATA/DesktopPet/config/balance.toml"
```

Fling the egg at the wall and it bounces back; drop it on the shelf and it slides along. How bouncy a hit is takes
the bouncier of the two, and how grippy is the average of both, so the egg, a little bouncy itself, still bounces
off a wall that isn't.

Obstacles are drawn as faint outlines, and clicks go through them. Positions are in physical pixels from the Home
monitor's top-left corner. The files are read at startup, so restart the Pet after editing them; a file with mistakes
is ignored and the log lists each one.

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
