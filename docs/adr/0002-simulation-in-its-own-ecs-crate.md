# The simulation runs on Bevy's ECS in its own crate, and its rules stay pure

The rules of the Pet's world (behaviour, XP, Forms, physics, combat) are plain functions and types in `pet-core`, with no
Bevy. A new crate, `pet-sim`, stores those types as ECS components and runs the rules as thin systems. It depends on
Bevy's ECS, app and time crates but not on rendering, so the compiler stops it from reaching the renderer. The renderer
in `pet-app` only reads components and messages, and nothing flows back to the simulation. The simulation also owns the
Animation clock, so attack timing, click picking and the Hit mask all match the Frame that is drawn. A tick runs in
fixed phases: Sense, Interpret, React, Decide, Act, Resolve, Present. Inside a phase, independent systems subscribe to
messages, so a new feature adds a system and edits nothing else. Only OS events, AI replies and save writes are truly
asynchronous.

We measured before choosing ([simulation-architecture.md](../research/simulation-architecture.md)). Plain Rust and the
ECS were within noise of each other at 200 enemies and close even at 100,000, so speed didn't decide it. A plain-Rust
simulation would have needed its own entity storage, spawning, thread pool and mode switching, plus a copy of every Actor
for drawing. A simulation written straight in `pet-app` would have tied the rules to the renderer. This trades away the
old PRD line that swapping Bevy must not touch the core: the rules would survive a swap, but `pet-sim` would need
rewriting.

## Consequences

- Bevy upgrades reach `pet-sim`'s systems, so they stay thin.
- Order between systems must be stated with `.chain()` or sets, and ambiguity detection should be on, or parallel
  systems can run in any order.
- Bevy drops a message after two updates whether or not it was read, so a system that is switched off misses what was
  sent meanwhile.
- Bevy's fixed clock can run a burst of steps on waking from Hidden, so the simulation must hold it to keep the freeze.
