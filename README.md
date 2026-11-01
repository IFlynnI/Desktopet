# Desktop Pet

A desktop companion that lives on top of your other Windows apps. See the [PRD](docs/PRD.md) for what it is and
where it's going, and the [glossary](GLOSSARY.md) for the words used here.

## Run it

```bash
cargo run -p pet-app
```

Press Ctrl+Alt+P or left-click the tray icon to hide or show the Pet. To stop it, right-click the tray icon and
choose Quit.

## Test it

Fast tests have no window, GPU or mouse:

```bash
cargo nextest run
```

Only one Pet can run at a time, so quit yours before running the next two. Quiet tests run the real app but leave
your mouse alone, so you can keep working:

```bash
cargo e2e-quiet
```

The full set also takes over your mouse and opens a magenta window, so keep your hands off it. It stops with a clear
message if the mouse moves:

```bash
cargo e2e
```

A plain `cargo test` runs the fast tests only. Install the runner once with `cargo install cargo-nextest --locked`.

Anything that can't be automated is in [docs/manual-checks.md](docs/manual-checks.md).
