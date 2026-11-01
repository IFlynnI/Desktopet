# Desktop Pet

A desktop companion that lives on top of your other Windows apps. See the [PRD](docs/PRD.md) for what it is and
where it's going, and the [glossary](GLOSSARY.md) for the words used here.

## Run it

```bash
cargo run -p pet-app
```

Press Ctrl+C in that terminal to stop it.

A release build (`--release`) has no console, so Ctrl+C can't reach it, and `cargo run` waits for it forever. There's
no Quit in the app yet, so stop a release build with:

```bash
taskkill /IM desktop-pet.exe /F
```

## Test it

Fast tests have no window, GPU or mouse:

```bash
cargo nextest run
```

Tests against the real app take over your mouse for about 40 seconds and open a magenta window, so keep your
hands off it:

```bash
cargo e2e
```

A plain `cargo test` runs the fast tests only. Install the runner once with `cargo install cargo-nextest --locked`.

Anything that can't be automated is in [docs/manual-checks.md](docs/manual-checks.md).
