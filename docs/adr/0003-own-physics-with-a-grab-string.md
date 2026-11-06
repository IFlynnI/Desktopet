# The Pet's physics is our own code in a `pet-physics` crate, with upright boxes and a springy grab string

Dragging must be momentum-based. The owner grabs the Pet, moves the cursor, and can flick or swing it to throw it.
Surfaces need a lot of freedom over grip and bounce: a wall the Pet slides down or bounces off, a bouncy prop, and a
"hero landing" that never bounces, even on a bouncy floor. So the grab is a short spring-damper string from the
cursor to the point where the owner grabbed the Pet. The swing comes from that string, as the Pet orbits the cursor,
so the Body stays an upright rectangle and never rotates. The art, the Hit mask, the shadow and the Emotes are
therefore unaffected. Every surface and every Body has a Material, which sets friction and bounciness. A combine rule
decides what happens when two Materials touch, but an Action's override always wins, so a hero landing really stops
dead. Fast movers are swept against Obstacles so they can't pass through thin walls. While the Pet is held or
falling, the Mind may choose among the Actions that say they can run in that Situation. When none suits, the
Situation's Animation plays as before.

The rules live in a new crate, `pet-physics`, which has no dependencies and owns the geometry types that `pet-core`
used before. `pet-core` depends on it, so the Mind's planner can ask "if I do this, where do I land", and `pet-sim`
runs it each tick. A crate rather than a module makes the compiler stop physics from reaching into the Mind, and
gives physics its own tests and benchmarks.

We measured the alternatives headless on the reference machine
([physics-rotation.md](../research/physics-rotation.md)):

| | Our own code | Rapier 2D 0.36.1, used directly | Avian 2D 0.8.0 |
|---|---|---|---|
| One Pet, per 30 Hz tick | 0.2 µs | 3 µs | about 0.66 ms |
| 1,000 boxes, per tick | 190 µs | 790 µs, with 2.2 ms at p95 | not run |
| 10,000 boxes, per tick | 1.9 ms | 69 ms, with 245 ms at p95, so ticks are dropped | not run |
| Added to the exe | about 30 KB, no crates | 1.7 MB, 46 crates, about 300 `unsafe` | 10–13 MB |
| Fits ADR-0002 | yes | yes, it needs no Bevy | no: it only runs inside a Bevy `App`, so the rules would move into `pet-sim` |
| A per-Action override of the combine rule | a plain check before the rule | needs a contact hook | needs a contact hook |

Our own code did everything that was asked:
- A swung Pet left at 27 m/s.
- The swept test let 0 of 200 fast boxes through a thin wall, against 142 without it.
- A hero landing on a floor with 0.9 bounciness bounced back at exactly 0 m/s, against 12.4 m/s without the override.
- Runs were bit-identical across repeats and across processes.

Rotating the box while drawing it upright added only a mismatch: the art sank or hovered by up to 15 art pixels, and
there was no feel the string doesn't already give.

This amends the decision of 2026-11-03. No physics engine, rectangles, 30 ticks a second, no rotation and swept tests
all still hold. "Nothing hangs on joints" changes: the grab string is a soft joint, applied as a force. "Nothing
stacks" changes a little: the Pet may stand on a Prop, and Props may rest on each other.

## Consequences

- Release speed is capped by a balance number, starting near 2,500 px/s, because a hard swing reached about
  2,700 px/s.
- The string's length and stiffness, gravity, friction and bounciness are feel numbers. They are tuned by hand and
  never pinned by a test.
- Physics works in physical screen pixels. The Body is scaled from art pixels, so a throw feels the same at every
  scale.
- Determinism is proven on one machine only. Cross-machine replays aren't promised.
- Choosing an Action while held or falling is how air attacks or bracing for a landing become possible, without a
  Situation that is special to one of them.
