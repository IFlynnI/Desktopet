# Physics for a held, swung and thrown Pet: facts and a recommendation

**Researched:** 2026-11-06, for Bevy 0.20.1, avian2d 0.8.0, rapier2d 0.36.1, bevy_rapier2d 0.37.0, rustc 1.97.1, on
Windows 11 with a Ryzen 9 3950X (32 logical processors).
**Purpose:** the owner's new wishes for dragging and surfaces reopen the 2026-11-03 decision, "No physics engine;
rectangles for the world and combat, pixels for clicks; 30 ticks a second; nothing rotates, stacks or hangs on joints".
This document measures the options and recommends one.

**The ask, as narrowed on 2026-11-06.** The art never rotates. Angular momentum is wanted only so that flicking the
cursor builds up throw momentum, like whipping or slinging. So the main candidate is a Body that stays an upright
rectangle, held by a spring from the cursor, which can be swung round the cursor in an arc and let go like a sling.
Surfaces need friction and bounciness per surface and per Body, with combine rules and per-Action overrides (a
zero-bounce hero landing, a wall slide). Fast things must not pass through thin Obstacles. The same inputs must give the
same result, for tests, replay and the teachable Mind. The RPG mode needs 1,000 enemies normally and 10,000 at most.
**Drawing a rotated Pet as pixel art is out of scope**, because the art stays upright.

**How to read the marks**, as in [simulation-architecture.md](simulation-architecture.md):

| Mark | Meaning |
|---|---|
| **Verified** | Read in the crate source of that exact version, or in its docs or changelog. |
| **Measured** | Run on this machine for this document. |
| **Inference** | My reasoning from verified or measured facts. Not run. |
| **Not verified** | I found no source that settles it. |

## Bottom line

- **Recommendation: hand-rolled, in `pet-core`.** Use upright boxes, a string from the cursor, a swept test against
  Obstacles, and materials with combine rules and an override. This is option A below. Keep "no physics engine".
- **It does everything asked, measured.**
  - It swings the Pet, which leaves at 27 m/s.
  - In a tunnelling test, none of 200 fast boxes escaped through a thin wall. Without its swept test, the same code let
    142 escape.
  - A hero landing on a bouncy floor bounced back at 0.000 m/s, against 12.4 m/s without the override.
  - It gives bit-identical results run after run.
- **It is the cheapest by far, measured.** One Pet costs 0.2 µs a tick. 1,000 bodies cost 190 µs a tick on average
  (0.6% of a core), and 10,000 cost 1.9 ms (5.8% of a core, one thread). It adds about 30 KB to the program, needs no
  new crates, and builds in seconds.
- **Rapier, used directly, is the best engine fit**, but it isn't needed. `rapier2d` has no Bevy dependency, so
  `pet-core` could use it. One Pet costs 3 µs a tick. 1,000 bodies cost 790 µs on average with a 2.2 ms p95. 10,000
  cost 69 ms on average with a 245 ms p95, in a deliberately overlapping crowd. Under A, enemies wouldn't be engine
  bodies anyway. It adds 1.7 MB and 46 crates. Measured.
- **Avian is the worst fit here.** It works only inside a Bevy `App`, so the rules would move from `pet-core` into
  `pet-sim`, against [ADR-0002](../adr/0002-simulation-in-its-own-ecs-crate.md). One Pet costs about 0.25 ms of wall
  time and 0.66 ms of CPU a tick, about 2% of a core in companion mode, after subtracting the empty `App`. Measured.
- **A body that really rotates but is drawn upright adds nothing a player would see, and some things they would.**
  The sling comes from the string, not from spin. Spin took speed out of the throw: Rapier's release speed fell from
  35.2 to 27.7 m/s while the box spun at 40 rad/s. And the art would hover or sink by up to 15 art pixels, because the
  rotated box no longer matches the upright art. See [Rotation](#does-real-rotation-add-anything-a-player-would-feel).
- **Combine rules alone can't express a per-Action override.** In Avian, `Max` outranks every other rule, so a bouncy
  floor set to `Max` always wins. In Rapier, `GeometricMean` outranks `Max`. So a Pet with zero bounce and that rule
  happens to force zero bounce, but it can't force any other value. Both engines need a contact hook for a general
  override. Our own code needs one `Option`. Verified.

## Terms

| Term | Meaning |
|---|---|
| **Rigid body** | A shape that moves and collides but never bends. |
| **AABB** | Axis-aligned bounding box: an upright rectangle that never rotates. Our Body is one. |
| **Impulse** | An instant change of velocity. Contacts are solved by applying impulses. |
| **Restitution** | Bounciness: the share of approach speed kept after a hit. 0 sticks, 1 bounces fully. |
| **Friction** | How much a contact slows sliding along the surface. |
| **Combine rule** | How two materials' values become one for a contact: average, min, max or multiply. |
| **Override** | A value set by the current Action or Situation that wins over both materials and the rule. |
| **Tunnelling** | A fast Body jumping past a thin Obstacle between two ticks, never touching it. |
| **Swept test / CCD** | Checking the whole path a Body travels in a tick, not just where it ends. CCD is "continuous collision detection". |
| **Spring-damper** | A pull that grows with stretch (the spring) and resists stretching speed (the damper). A string is a spring that only pulls once longer than its rest length. |
| **Joint** | A constraint that links two bodies. A spring applied as a force is the soft, simple kind. |
| **Contact hook** | Your own code that an engine calls for each contact, so it can change friction and bounce. |
| **Deterministic** | The same inputs give exactly the same bits out. |

## The main candidate: an upright Body on a string

```
   cursor ●───────string (rest length 0.3 m)───────▶ grab point ┐
                                                      ┌──────────┘
                                                      │  Body: an upright rectangle that never spins
                                                      └─ Hit mask, shadow and Emotes stay as they are
```

- Each tick the string pulls the grab point towards the cursor. It pulls only once it is longer than its rest length,
  with a spring stiffness of 150 × mass and damping along its length. That is 40 lines in `pet-core`.
- The grab point is fixed relative to the Anchor, because the Body never turns. So the pull moves the whole Body, and no
  torque is involved.
- Circling the cursor makes the Body orbit it. That orbit is the angular momentum the owner wants, about the cursor
  rather than about the Pet's own centre. On release the Body keeps the orbit's speed, along the tangent, like a sling.
  **Measured:** two quick 0.5 m circles of the cursor, then release, threw the Pet at 27.2 m/s, about 2,700 px/s.
- **Watch the tuning.** The cursor circled at 12.6 rad/s, almost exactly the string's natural 12.2 rad/s, so the swing
  resonated. That's why the throw is 4 to 5 times the cursor's speed. Peaks reached 42 to 66 m/s in the various
  engines. So don't read the engines' different release speeds as quality. The owner needs a speed cap. Measured, and
  the resonance is Inference from √(k/m).
- Because nothing rotates, the Hit mask, shadow, Emotes and the Anchor's meaning are all unchanged. While airborne the
  Anchor is still where the feet are, and the Body hangs below the grab point. Inference.

## Does real rotation add anything a player would feel?

The comparison case is a body that really rotates in the physics, grabbed off-centre, but is drawn upright.

| Measured, same cursor path | Release speed | Peak spin |
|---|---|---|
| Hand-rolled, upright | 27.2 m/s | none |
| Rapier, rotation locked | 35.2 m/s | none |
| Rapier, rotation free | 27.7 m/s | 39.7 rad/s (6 turns a second) |
| Avian, rotation locked | 35.7 m/s | none |
| Avian, rotation free | 35.8 m/s | 142 rad/s (23 turns a second) |

- **The sling is there without spin.** The upright body was thrown just as hard. Measured.
- **Spin costs throw speed in Rapier,** where some of the pull went into turning. It also makes bounces depend on which
  corner lands first, which a player can't see on upright art. The two engines disagree by 3.6 times on how fast the
  same box spins. Measured, and the "can't see" is Inference.
- **The art and the box stop matching.** The Pet is 50 × 80 art pixels. Lying on its side, the box is 50 tall, so
  upright art would sink 15 art pixels into the floor, 30 screen pixels at 2×. At 45° the box stands (25 + 40) / √2 ≈ 46
  pixels above its centre, so the art would hover 6 pixels. Inference, by arithmetic.
- **So real rotation adds a mismatch, not feel.** The string's rest length and stiffness are what a player feels.

## The options, measured

| | A. Hand-rolled AABB in `pet-core` | C1. `rapier2d`, no Bevy | B. Avian 0.8 in a Bevy `App` | C2. `bevy_rapier2d` 0.37 |
|---|---|---|---|---|
| Added binary size, release | +30 KB, spike scenarios included | +1.7 MB | +10.1 to +12.7 MB | +6.3 to +6.5 MB |
| Clean build, release with thin LTO (baselines 2.2 s and 71 s) | 2.6 s | 76 s (rapier2d 34 s, parry2d 12 s, wide and nalgebra 7 s each) | 296 s (avian2d 152 s, the final link 105 s, parry2d 13 s) | 163 s (bevy_rapier2d 53 s, rapier2d 34 s, the final link 75 s) |
| Rebuild after an edit, release with thin LTO (baselines 1.7 s and 14 s) | 2.1 s | 22 s | 101 s | 75 s |
| New crates | 0 | 46 | 42 over Bevy | 27 over Bevy |
| 1 Pet + 20 Obstacles, mean tick | 0.2 µs | 3.1 µs | 330 µs wall, 750 µs CPU (with the empty `App`'s 78 and 87) | not run |
| 1,000 boxes, mean / p95 tick | 190 / 230 µs | 790 / 2,170 µs | 1,600 / 2,500 µs wall, 4,000 µs CPU | 1,420 / 2,830 µs wall, 2,190 µs CPU |
| 1,000 boxes, share of one core | 0.56% | 2.4% | 11 to 13% | 6.6% |
| 10,000 boxes, mean / p95 tick | 1.9 / 2.3 ms | 69 / 245 ms | 9.6 / 11.8 ms wall, 36.5 ms CPU | 74 / 255 ms |
| 10,000 boxes, share of one core | 5.8% | 204%, so ticks are dropped | 110%, spread over threads | 221%, ticks dropped |
| Heap at peak: 1 Pet / 1,000 / 10,000 | 1 KB / 70 KB / 0.7 MB | 43 KB / 3.5 MB / 41 MB | 3.3 MB / 6.0 MB / 38 MB above the empty `App` | 7.0 MB / 66 MB above the empty `App` (1,000 / 10,000) |
| Private memory at peak, whole process | 0.9 / 0.8 / 1.4 MB | 1.0 / 5.3 / 44 MB | 9.5 / 14.5 / 61 MB, where the empty `App` is 3.7 to 5.8 MB | 13.7 / 75 MB |
| Tunnelling, 200 boxes at 10 to 50 m/s into a 0.2 m wall | 0 escaped (142 without the swept test) | 0 escaped, with CCD on or off | 0 escaped in the 1,000 run | 0 escaped in the 1,000 run |
| Hero landing override | Yes, run: 0.000 m/s bounce | Yes, with a hook, run: 0.000 m/s | Yes, with a hook. Read in the source, not run | Same hook as rapier. Not run |
| Same result every run | Yes, in-process and across 2 processes | Yes, both | Yes, both, on 32 threads | Yes, in-process |
| Rules stay in `pet-core`, ADR-0002 (Verified) | Yes | Yes, as `pet-core`'s first dependency | No, they move into `pet-sim` | No |

All rows are **Measured** except where a row or cell says otherwise; crate counts come from `cargo tree`. The engines'
"0 escaped" counts because the same 1,000-box run let 529 escape from A without its swept test. The size range for B
and C2 is the `.exe` minus a lean and a feature-merged Bevy baseline; the lower figure is closer to what `pet-app`, which
already has most of Bevy, would see. D, the hybrid, has no column of its own; see [Option D](#option-d-the-hybrid).

**Why Rapier slows down so much at 10,000.** 10,000 boxes of 0.24 m² fill 2.5 times the 960 m² room, and all start
fast. The bodies don't collide with each other here, but Rapier's broad phase still walks every overlapping pair before
its group filter drops them. Verified in `broad_phase_bvh/update.rs`. The hand-rolled code never looks at body pairs at
all. Turning Rapier's CCD off cut 10,000 to 37 ms on average with a 108 ms p95, still over budget. So the comparison is
unfair to engines at 10,000, but it is the shape of a horde walking on one ground line. Measured, and the cause is
Inference.

**Why Avian costs so much for one Pet.** Its plugins run dozens of systems every step, on six substeps, through the
multi-threaded executor. CPU is about twice the wall time, so the work is spread over threads. The single-threaded
executor and turning off the `parallel` feature weren't tried. Measured, and the cause is Inference.

## How it was measured

- **Where:** a throwaway workspace in `%TEMP%\desktop-pet-physics-spike`, outside the repo, with the repo's release
  profile (`codegen-units = 1`, thin LTO). It is now deleted. Nothing in the repo's Cargo files changed.
- **One world for every engine,** in metres at 100 px a metre, 30 ticks a second, gravity 9.81. It is a 40 × 24 m room:
  four walls 0.2 m thick, with a slippery left wall and a bouncy right wall, plus 16 ledges 0.1 m thick. That's 20
  static Obstacles.
- **1 Pet:** a 0.5 × 0.8 m box, grabbed 0.2 m right of and 0.3 m above its centre. It is swung for 60 ticks, then
  free for 540, 20 runs.
- **Many boxes:** N boxes of 0.4 × 0.6 m at seeded random places, flying at 5 to 40 m/s, so many cross a wall's
  thickness in one tick. Rotation is locked. They hit Obstacles but not each other, as enemies would. Sleeping is off
  and CCD on, for 300 ticks.
- **Timing:** wall time of each tick, and the process's CPU across all threads, as a share of one core at 30 Hz. The
  Bevy variants include their `App`'s own update. An empty `App` costs 78 µs wall and 87 µs CPU a tick, 0.26% of a
  core.
- **Memory:** a counting allocator gives the heap. The process's peak private bytes are what Task Manager calls
  "Memory".
- **Size and build time:** each variant was built clean in its own target directory. The size is its `.exe` minus a
  matching baseline: plain Rust for A and C1, and the same headless Bevy `App` for B and C2. The relink time is a
  rebuild after touching `main.rs`. A shared directory merges Bevy features across packages, which gives the lower size.
- **Determinism:** each scenario hashed every body's position bits. It was repeated in one process and again in a
  second process. The hashes matched.

## Materials, combine rules and overrides

| | How a surface and a Body get materials | Combine rules, and who wins a clash | A per-Action override |
|---|---|---|---|
| A | `Material { friction, restitution, rules }` on each Obstacle and Body | Our choice. The spike uses Average < Min < Multiply < Max | `ContactOverride { friction: Option, restitution: Option }` on the Body, set by the Action. Wins outright |
| B. Avian | `Friction` and `Restitution` components, each with a `CoefficientCombine` | Average, GeometricMean, Min, Multiply, Max. Clash priority: Max > Multiply > Min > GeometricMean > Average | `CollisionHooks::modify_contacts`, plus `ActiveCollisionHooks::MODIFY_CONTACTS` on the Pet, writing a manifold's `friction` and `restitution` |
| C. Rapier | `ColliderBuilder::friction` and `restitution`, each with a `CoefficientCombineRule` | Average, Min, Multiply, Max, ClampedSum, GeometricMean. In a clash, the later one in that list wins | `PhysicsHooks::modify_solver_contacts` with `ActiveHooks::MODIFY_SOLVER_CONTACTS`, writing `ctx.rigid_mut()`'s `friction` and `restitution` per manifold |

Rows B and C are **Verified** in `physics_material.rs` (Avian) and in `coefficient_combine_rule.rs` and
`physics_hooks.rs` (Rapier). The Rapier hook and A were also **Measured**. A 10 m drop onto a floor with restitution 0.9
and the `Max` rule hit at 13.4 m/s. It bounced back at 12.4 m/s without the override, and at 0.000 m/s with it. Wall
slides are the same mechanism with `friction: Some(0.02)`.

## Swept tests and tunnelling

- **A:** each tick, the Body's box moves along its path. It stops at the first Obstacle it would touch, bounces and
  slides by the materials, then goes on for the rest of the tick, up to four times. Obstacles are only static, so this
  is exact for upright boxes. Measured: 0 of 200 escaped. The same code without the sweep let 142 of 200 escape, and
  529 of 1,000 in the many-boxes run.
- **B. Avian 0.8:** swept CCD is on by default for fast dynamic bodies against static and kinematic ones. Speculative
  contacts are opt-in through `SpeculativeCcd`. `SweepMode::Linear` ignores spin, and `NonLinear` covers it. Verified
  in `dynamics/ccd/mod.rs`.
- **C. Rapier:** `ccd_enabled(true)` per body. Measured: nothing escaped even with CCD off. The source credits a
  larger speculative margin and "one-sided static geometry". That mechanism is Not verified.

## Determinism

- **A:** Measured identical across repeats and processes. It is cross-platform by construction: it uses only `+ - * /`,
  `sqrt`, `min` and `max`, with no trigonometry. Rust never fuses a multiply and an add on its own. Inference: IEEE 754
  fixes these results bit for bit on every mainstream CPU.
- **C. Rapier:** "locally deterministic" by default. It is cross-platform with `enhanced-determinism`, if insertion
  order is identical and the CPU follows IEEE 754. The `parallel` solver gives the same results as the sequential
  one, but `simd8` can't be combined with it. Verified,
  [rapier.rs determinism guide](https://rapier.rs/docs/user_guides/rust/determinism). Measured here: the same hashes
  with and without the feature. That means the feature wasn't exercised on one machine. It doesn't prove anything
  across machines.
- **B. Avian:** `enhanced-determinism` switches to `libm` maths. A cross-platform hash test runs on CI for every pull
  request. Verified in `src/tests/determinism_2d.rs`. Measured: the feature left the upright runs unchanged, but
  changed the rotating Pet's hash. Only rotation calls trigonometry.
- **Every option needs** a fixed tick, the same spawn order and inputs recorded per tick. The cursor path is an input.

## Architecture fit

- **A** is plain Rust in `pet-core`, and `pet-sim` stays a thin adapter. Nothing changes in the layering.
- **C1, `rapier2d`,** has no Bevy in its tree (measured with `cargo tree`), so `pet-core` could own a `PhysicsWorld`.
  But it would be `pet-core`'s first dependency. Its 46 crates include nalgebra, parry2d, simba and wide, and its source
  has about 300 `unsafe` uses. `pet-core` is described as having "no unsafe code"; that is about its own code, but it is
  the owner's call. Bodies live in Rapier's own sets, so `pet-sim` mirrors them to entities by handle.
- **B, Avian,** is ECS-native. Its bodies are components, it pulls in the `bevy` umbrella crate and `Transform`, and its
  hooks are Bevy `SystemParam`s. So the physics rules would live in `pet-sim`, not `pet-core`. That contradicts
  ADR-0002's "rules stay pure", and tests would need an `App`.
- **C2, `bevy_rapier2d`,** has the same problem as B. It also needs `TimestepMode::Fixed { dt: 1/30 }` set by hand.
  Its default is variable with at most 1/60 s a step, which would run a 30 Hz tick at half speed. Verified in
  `plugin/configuration.rs`.

## Option D, the hybrid

D means an engine for the Pet and props, plus plain AABB rules for enemies. A already covers the Pet, so D pays off
only if props must rotate, pile up or hang on real joints. It would then cost Rapier's 1.7 MB and 46 crates, plus two
collision models that must agree on the same Obstacles. Inference.

## Code size and risk of A

- **The spike's module is 194 lines.** Its 171 non-blank lines cover upright boxes, the swept test, materials, combine
  rules and the override. The string is 25 more lines. Measured with `wc`.
- **Still missing:**
  - Body against Body, for a bouncy prop the Pet can hit or stand on (PRD item 2, ball and yarn), about 80 lines.
  - Emitting held, falling and landed, about 60 lines.
  - A grid once Obstacles number in the hundreds, about 50 lines, reusing the spatial grid.
  - A speed cap, about 10 lines.
- **With tests, that's about 450 lines of code and 500 of tests.** Inference.
- **Risk is low for the upright, static-Obstacle part.** The maths is short and every step is testable.
  - Body-against-Body resting contact is moderate: a Pet standing on a moving prop, or two props stacked. Stacks of
    many bodies are where hand-rolled solvers go wrong. A full rigid-body solver with rotation would be about 1,500 to
    2,500 lines and high risk; that was the original option A. Inference.

## The 2026-11-03 decision: what holds and what changes

| Part | Under the recommendation |
|---|---|
| No physics engine | **Holds.** |
| Rectangles for the world and combat, pixels for clicks | **Holds.** The Body stays an upright rectangle. |
| 30 ticks a second | **Holds.** |
| Nothing rotates | **Holds**, now by choice rather than by limitation. |
| Nothing stacks | **Changes a little**, if the Pet may stand on a prop. Piles of bodies stay out. |
| Nothing hangs on joints | **Changes.** The grab string is a soft joint, but applied as a force, with no constraint solver. |
| A swept test stops fast Projectiles passing through | **Holds**, and now covers the thrown Pet and every Body against Obstacles. |
| New | Materials with combine rules per Obstacle and Body, a per-Action override, a speed cap. |

## Recommendation

Take A. It meets every point of the narrowed ask, as measured above, with 4 to 21 times less CPU than the engines at
1,000 bodies. It adds no crates, keeps ADR-0002 as written, and is deterministic across machines without a feature
flag. If rotating props or real joints are wanted later, `rapier2d` used from `pet-core` is the fallback: it is the
only engine that fits the layering, and the Body and Material types can stay the same in front of it.

## Open decisions for the owner

1. **The string's feel:** its rest length (0.3 m here), its stiffness, and whether it can push as well as pull.
2. **A speed cap for throws:** the spike reached 2,700 px/s at release and 6,600 px/s at peak.
3. **Can the Pet stand on a prop**, and can props rest on each other? That decides the Body-against-Body work.
4. **The combine-rule list and its clash order**, and whether the override also covers enemies.
5. **Whether a thrown Pet's Strike box** uses the same speed threshold as before (architecture "Thrown Pet").
6. **If an engine is ever taken,** is a dependency with `unsafe` inside acceptable in `pet-core`?

## Not verified

- Avian with the single-threaded executor or without `parallel`. That might cut its one-Pet cost.
- Avian's hero-landing hook and `bevy_rapier2d`'s 1-Pet cost. They were read in the source, not run.
- Determinism across machines, for any option. Only one machine was available.
- Why Rapier didn't tunnel with CCD off.
- Build times in the dev profile, which has no LTO. Only release builds were timed.
- Feel. Nothing here was drawn on screen, by request. The release speeds are numbers, not play-tests.

## Sources

- avian2d 0.8.0: [docs.rs](https://docs.rs/avian2d/0.8.0/avian2d/). The version table maps Bevy 0.20 to Avian 0.8
  ([README](https://github.com/avianphysics/avian)). Source read: `physics_material.rs`, `collision/hooks.rs`,
  `dynamics/ccd/mod.rs`, `tests/determinism_2d.rs` and `lib.rs` (feature table).
- rapier2d 0.36.1: [docs.rs](https://docs.rs/rapier2d/0.36.1/rapier2d/),
  [CHANGELOG](https://github.com/dimforge/rapier/blob/master/CHANGELOG.md) (v0.36.0 moved bevy_rapier into the repo
  and hook contact fields behind `rigid_mut()`), [determinism guide](https://rapier.rs/docs/user_guides/rust/determinism).
  Source read: `coefficient_combine_rule.rs`, `physics_hooks.rs`, `broad_phase_bvh/update.rs`, `integration_parameters.rs`.
- bevy_rapier2d 0.37.0: [docs.rs](https://docs.rs/bevy_rapier2d/0.37.0/bevy_rapier2d/). It depends on rapier2d 0.36.1
  and bevy 0.20. Source read: `plugin/configuration.rs`.
