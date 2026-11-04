# Simulation architecture: facts for the design decision

**Researched:** 2026-11-02, for Bevy 0.20.1, wgpu 30.0.1, rustc 1.97.1 on Windows 11.
**Purpose:** give the facts behind how the domain simulation gets built, and how many enemies it can handle, including
the real trade-off between plain Rust and Bevy's ECS. This document has facts only. It doesn't recommend a design.

**What the owner asked for, 2026-11-02.** Companion mode, the Pet alone beside other apps, must stay very light.
Encounters may use much more CPU across all cores, because the user is focused on the game. The first goal is 200
enemies, and hordes may go well beyond that. Some enemies fly, so not everything stays on one line. A boss may need far
smarter AI than a basic enemy. The attitude is pragmatic, not purist.

**How to read the marks.** Each fact has one:

| Mark | Meaning |
|---|---|
| **Verified** | Read in the 0.20.1 crate source, or in the docs.rs page for 0.20.1. Links go to the `v0.20.1` tag. |
| **Measured** | Run on this machine for this document. |
| **Inference** | My reasoning from verified facts. Not run. |
| **Other version** | True of a different Bevy version. Don't assume it holds for 0.20.1. |
| **Not verified** | I found no source that settles it. |

Context7 wasn't available in this session. The source of truth is the crate source in
`C:\Users\flynn\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f`, which is exactly what docs.rs builds from.
Links are the sources.

## Bottom line

- **At the first goal of 200 enemies, performance doesn't separate plain Rust from ECS.** On one thread both stay
  under 0.05% of a core, on the line or in 2D. The gap is 1 to 2 µs in a 33,333 µs tick. Threads make 200 slower: 5 µs
  becomes 59 µs. [Benchmark](#benchmark).
- **At 100,000 they're still close.** Walkers on one thread: plain 6.1 ms, ECS 6.4 to 8.0 ms. On 8 threads with a
  long-lived pool: plain 2.6 ms, ECS 2.7 to 3.1 ms. Flyers on 8 threads: 4.6 to 4.7 ms for all three. Threads start to
  pay off somewhere between 1,000 and 10,000.
- **Threads cut the wait, not the CPU.** 100,000 flyers use 48% of a core on one thread and about 78% on 8.
- **A mirror is cheap.** Copying 100,000 positions and action ids takes 405 µs in matching order, or 795 µs (2.4% of a
  core) by entity, which is the figure that survives despawning. At 200 it takes 0.4 to 1.4 µs.
- **Disabled encounter systems cost almost nothing.** 30 systems behind false `run_if` conditions don't rise above the
  ±0.03% noise. With no conditions and empty queries, the multi-threaded executor costs about 0.5% of a core.
- **`ExecutorKind` doesn't exist in 0.20.1.** The API is `Schedule::set_executor`, per schedule.
- **All simulation figures are floors:** no spawning, utility scoring or rendering.
- **Drawing is the larger cost, and it's CPU, not GPU.** With the app's render setup, all 48 combinations held 30 fps,
  up to 10,000 sprites at 512×512. 10,000 moving, animated sprites take 20 to 23 ms of a 33 ms frame and 72 to 83% of
  one core, at any size. The GPU peaked at 50%. [Drawing cost](#drawing-cost-measured).
- **The cost per sprite is about 1.9 µs a frame moving and animated, and about 0.8 µs still.** Size barely changes it.
  So roughly 14,000 to 16,000 moving sprites would fill 33 ms on this machine. Inference, from the measured slope.
- **Bevy 0.20 replaced the sprite renderer.** Older batching advice is out of date. **Bindless is off for sprites on
  the RTX 2080 Ti.** Bevy's own check says no, because it counts the texture as a sampler as well. So different textures
  and different atlas frames do split batches here. Measured, and the reason Verified in the source.
- **`bevy_ecs` works alone**, with 55 crates and no window or GPU code. It has no clock and no fixed timestep.
- **`FixedUpdate` catches up, and the catch-up is bounded.** One update adds at most 250 ms, so at most 16 steps at the
  default 64 Hz. Waking from Hidden can trigger exactly that burst.
- **Buffered messages are dropped by update count, not by who read them.** With no reader they go at the same time.
- **`big-brain` has no release for Bevy 0.20.** Its data shape is easy to copy from the sources in question 5.

## Terms

| Term | Meaning |
|---|---|
| **ECS** | Entity Component System, how Bevy stores game state. |
| **World** | The ECS container that holds all entities and data. |
| **System** | A plain function that runs over ECS data. |
| **Schedule** | An ordered set of systems. `schedule.run(&mut world)` runs them once. |
| **Tick** | One step of the simulation. |
| **Update** | One pass of Bevy's whole main schedule, usually one frame. |
| **Overstep** | Time banked towards the next fixed step. |
| **Executor** | The part of a schedule that decides which system runs when, on one thread or several. |
| **Run condition** | A check added with `run_if`, deciding whether a system runs this time. |
| **Task pool** | A set of threads kept alive, waiting for work. |
| **Change detection** | Bevy records a tick number each time a component is written, so systems can ask what changed. |
| **Table** | Bevy's storage for entities with the same components: one array per component. |
| **Sync point** | A place in a schedule where queued `Commands` are applied. |
| **Mirror** | Copying simulation results into ECS components each tick, so the renderer can read them. |
| **Uniform grid** | A 2D grid of equal cells. To find something nearby, look only in nearby cells. |
| **Draw call** | One instruction to the GPU to draw something. |
| **Batch** | Many sprites drawn by one draw call. |
| **Bind group** | The set of textures and buffers a draw call can see. Changing it splits a batch. |
| **Bindless** | One big bind group holds many textures. The shader picks one by index, so sprites with different textures can share a draw call. |
| **Slab** | Bevy's name for one such big bind group. |
| **Texture atlas** | One image holding many animation frames. |
| **Utility AI** | Each action gets a score, and the score picks the action. |
| **Consideration** | One input to an action's score, such as distance or health. |
| **Response curve** | A function that reshapes a 0 to 1 input into a 0 to 1 score. |
| **Feature unification** | Cargo builds one copy of a crate with the union of the features its users ask for. |

## 1. `bevy_ecs` on its own

| Fact | Status | Source |
|---|---|---|
| It can be used as a standalone crate, with no `App`, window or renderer. | Verified | [README](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/README.md) |
| A 30-line program with a manual `World`, `Schedule` and one message type built and ran here. | Measured | Temp directory, `default-features = false, features = ["std"]` |
| Dependencies: 55 crates with only `std`, 68 with default features. Both counts include proc-macro crates. | Measured | `cargo tree -e normal` |
| 8 of those 55 are Bevy crates: `bevy_ecs`, `bevy_ecs_macros`, `bevy_ecs_macro_logic`, `bevy_macro_utils`, `bevy_platform`, `bevy_ptr`, `bevy_tasks`, `bevy_utils`. Default features add `bevy_reflect` and its derive crate. | Measured | Same |
| Neither tree has `wgpu`, `winit`, `naga`, `bevy_render`, `bevy_app` or `bevy_window`. | Measured | Same |
| You do need a manual `World` and `Schedule`. You call `schedule.run(&mut world)` yourself. | Verified | [`examples/events.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/examples/events.rs) |
| Without the `multi_threaded` feature, `Schedule` uses the single-threaded executor. | Verified | [`executor/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/executor/mod.rs) |
| `FixedUpdate` lives in `bevy_app` and `Time` in `bevy_time`, so neither comes with `bevy_ecs`. | Verified | [`main_schedule.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/main_schedule.rs), [`fixed.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_time/src/fixed.rs) |
| Messages need `MessageRegistry::register_message` and `message_update_system` in your own schedule, or the buffers never clear. | Verified, and Measured in the program above | `examples/events.rs`, [`messages.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/message/messages.rs) |

Project facts this touches:

- [PRD §7.2](../PRD.md) calls the domain core framework-free with no Bevy, and the
  [proposal](../proposals/item-2-architecture.md) says `pet-core` has no dependencies. Adding `bevy_ecs` to `pet-core`
  would change that line.
- `bevy_ecs` would need the same `=0.20.1` pin as `bevy`. Otherwise Cargo can build two copies, and their types don't
  mix. Inference.
- A whole-workspace build merges features, so `pet-core` would get `multi_threaded` from `pet-app`. `cargo test -p
  pet-core` on its own wouldn't. Inference, from Cargo's feature unification.

## 2. Messaging in 0.20.1

| | Buffered message | Triggered event |
|---|---|---|
| Trait | `Message` | `Event`, `EntityEvent` |
| Send | `MessageWriter::write` | `World::trigger`, `Commands::trigger` |
| Receive | `MessageReader::read` in a system | An observer, a system whose first parameter is `On<E>` |
| Register | `App::add_message` | `World::add_observer`, `EntityWorldMut::observe` |
| Storage | `Messages<M>` resource | None. The event is passed straight to observers |
| Style | Pull: read at a fixed point in the schedule | Push: observers run inside the `World::trigger` call |
| Suited to | Many messages at once. The trait's docs call this more efficient than observers for bulk work | One-off things that should run straight away |

All of this is **Verified** from [`message/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/message/mod.rs),
[`event/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/event/mod.rs) and
[`observer/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/observer/mod.rs).

Recent renames, because older tutorials will mislead:

- Bevy 0.17 split the old `Event` into `Message` and `Event`
  ([guide](https://bevy.org/learn/migration-guides/0-16-to-0-17/#event-trait-split-rename)). **Other version**, but the
  names match the 0.20.1 source.
- 0.20 moved the bundle type: `On<Add<A>>`, not `On<Add, A>`
  ([guide](https://github.com/bevyengine/bevy/blob/v0.20.1/_release-content/migration-guides/observer_event_matching.md)). **Verified.**
- The lifecycle events are `Add`, `Insert`, `Discard`, `Remove` and `Despawn`. `Replace` survives only as a doc alias of
  `Discard` ([`lifecycle.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/lifecycle.rs)). **Verified.**

**When an unread message is dropped.** `Messages` keeps two buffers. Each `update()` swaps them and clears the older
one, so a message is gone after two updates, whether or not anything read it. A reader that reads at least once per update
never misses one. A reader's cursor doesn't keep a message alive. **Verified**,
[`messages.rs`](https://docs.rs/bevy_ecs/0.20.1/bevy_ecs/message/struct.Messages.html).

**When `update()` runs in an `App`.** `message_update_system` runs in `First`, behind a condition. `TimePlugin` adds a
system in `FixedPostUpdate` that signals it, so an update only happens after at least one fixed pass. That keeps
messages alive for `FixedUpdate` readers even when a frame runs no fixed step. **Verified**,
[`update.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/message/update.rs),
[`bevy_time/lib.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_time/src/lib.rs).

| Situation here | What happens | Status |
|---|---|---|
| 30 Hz cap, 64 Hz fixed step | Every update has a fixed pass, so an unread message lasts about two frames, 67 ms. | Inference |
| Hidden, no updates | `First` doesn't run, so nothing is dropped. The drop counts updates, not seconds. | Inference |
| `Time<Virtual>` paused | No fixed pass means no signal, so `update()` stops. Messages written in `Update` pile up until it unpauses. | Inference, read from code, not run |
| `bevy_ecs` alone | Nothing clears unless you schedule `message_update_system`. The docs warn the buffers then grow without limit. | Verified |

## 3. Fixed timestep

| Fact | Status | Source |
|---|---|---|
| The default timestep is 64 Hz, 15,625 µs. | Verified | [`fixed.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_time/src/fixed.rs) |
| `FixedMain` runs zero or more times per update, between `PreUpdate` and `Update`. | Verified | `fixed.rs`, [`main_schedule.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/main_schedule.rs) |
| Each update adds the `Time<Virtual>` delta to overstep, then runs steps until less than one timestep is left. So it does catch up. | Verified | `run_fixed_main_schedule` in `fixed.rs` |
| `Time<Virtual>` limits each update's delta to `max_delta`, 250 ms by default. Time over that is lost, not banked. | Verified | [`virt.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_time/src/virt.rs) |
| The clamp exists for suspend and for lag spirals. Under sustained overload the game runs slower than real time but doesn't freeze. | Verified | `virt.rs` docs |
| A paused virtual clock gives delta 0, so `FixedUpdate` doesn't run. | Verified | `fixed.rs`, `virt.rs` |
| Tests can force exact steps: `TimeUpdateStrategy::ManualDuration(d)`, or `FixedTimesteps(n)` for exactly n steps per `App::update`. | Verified | [`lib.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_time/src/lib.rs) |
| `UpdateMode::Reactive` updates when `wait` has passed, or on enabled events. `reactive_low_power` ignores device events. | Verified | [`winit_config.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_winit/src/winit_config.rs) |
| `Duration::MAX` overflows the deadline, so winit waits with no timer at all. | Verified | [`state.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_winit/src/state.rs) |

What that adds up to here. The project uses `reactive_low_power` with 33.3 ms, and `Duration::MAX` while Hidden
([`render_config.rs`](../../crates/pet-app/src/engine/render_config.rs)).

| Case | Result | Status |
|---|---|---|
| 30 Hz cap, default 64 Hz step | 2.13 steps per update on average: 2, sometimes 3. A sim on the default step costs 2.13 times a 30 Hz sim. | Inference |
| 30 Hz cap, step set to 30 Hz | About 1 step per update, sometimes 0 or 2, because updates aren't exactly 33.3 ms apart and window events add extra ones. | Inference |
| Frames longer than 250 ms (below 4 fps) | The sim runs slower than real time. | Inference |
| Most steps one update can run | 16 at 64 Hz. 7 or 8 at 30 Hz. | Inference: `(250 ms + leftover) / timestep` |
| Reactive mode with a long gap | No update means no steps. The next update's delta is the whole gap, clamped to 250 ms. | Inference |
| Hidden | Nothing runs, as designed. | Verified, [`presence.rs`](../../crates/pet-app/src/engine/presence.rs) |
| Any wake-up after Hidden | That update's real delta is the whole pause, clamped to 250 ms. Up to 16 steps run in one burst, before any `Update` system, so before `apply_presence`. It happens on every wake-up, including one that leaves the Pet Hidden. | Inference |

The proposal says the world freezes while Hidden, so the Pet reappears where it was. A fixed-step sim would move on by
up to 250 ms on waking unless something holds it. Pausing `Time<Virtual>` stops the steps, with the message side effect
from question 2.

## 4. Sprite scale

**The renderer changed in 0.20.** [PR #25432](https://github.com/bevyengine/bevy/pull/25432), merged 2026-09-02, moved
`Sprite` onto `Mesh2d` plus a material. The old `queue_sprites` loop is still in the crate, but only `Text2d` uses it
([migration guide](https://github.com/bevyengine/bevy/blob/v0.20.1/_release-content/migration-guides/sprite_mesh_migration.md)).
Most online sprite batching advice describes that old loop. **Verified.**

### How batching works now

Each `Sprite` gets a shared unit quad (`Mesh2d`) and a `SpriteMeshMaterial`, cached by the `Sprite`'s fields and its
`Anchor` ([`sprite_mesh/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_sprite_render/src/sprite_mesh/mod.rs)).
**Verified.**

| Fact | Status | Source |
|---|---|---|
| Two neighbouring sprites merge if they match on pipeline, draw function, mesh and material bind group. `Transparent2d` keeps the default `AUTOMATIC_BATCHING = true`. | Verified | [`batching/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/src/batching/mod.rs), [`mesh2d/mesh.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_sprite_render/src/mesh2d/mesh.rs) |
| `SpriteMeshMaterial` is bindless. Its `#[data]` field doesn't need `BUFFER_BINDING_ARRAY`, only `TEXTURE_BINDING_ARRAY`. | Verified | [`sprite_mesh_material.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_sprite_render/src/sprite_mesh/sprite_mesh_material.rs), [`as_bind_group.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/macros/src/as_bind_group.rs) |
| wgpu's DX12 backend offers that only with Resource Binding Tier 3 and shader model 5.1. | Verified for wgpu 30.0.1 | [`dx12/adapter.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/dx12/adapter.rs) |
| Bevy also requires binding-array samplers of at least sampler count × slab limit, here 1 × 2048. | Verified | `as_bind_group.rs` |
| On the RTX 2080 Ti under DX12: `TEXTURE_BINDING_ARRAY` true, `BUFFER_BINDING_ARRAY` false, exactly 2048 binding-array samplers. So the check passes with no margin, and a GPU reporting fewer lands in the "without bindless" column. | Measured | Probe with wgpu 30.0.1 |
| Bevy asks for all adapter features unless told otherwise, and the project doesn't change that. | Verified | [`renderer/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/src/renderer/mod.rs), `render_config.rs` |
| A slab holds 2048 resources by default. Same-texture sprites reuse a slot. Per-material data doesn't count. It spills to a new slab at 2048 textures and samplers, or past 65,536 materials. | Verified | [`material_bind_groups.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/src/material_bind_groups.rs), [`bindless.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_render/src/render_resource/bindless.rs) |
| Default `Blend` sprites go in the sorted `Transparent2d` phase, so only neighbours in draw order can merge. `Opaque` and `Mask` go in binned phases, grouped regardless of order. | Verified | [`mesh2d/material.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_sprite_render/src/mesh2d/material.rs) |

So on the reference machine. All **Inference** from the rows above, not run in a Bevy app:

| Does it split a batch? | With bindless (this GPU) | Without bindless |
|---|---|---|
| A different texture per sprite | No, until a slab fills. Then only where sprites from two slabs interleave | Yes: each material has its own bind group |
| Atlas index changing every frame | No | Yes, wherever neighbours differ |
| Swapping the image every frame | No, until a slab fills | Yes |
| Different tint or flip | No | Yes |
| Different material type, such as a custom sprite shader | Yes: a different pipeline | Yes |
| A `Text2d` between sprites in draw order | Yes: a different draw function and renderer | Yes |

**Correction, 2026-11-03:** a running Bevy app on this machine reports bindless off for sprites, so the "Without
bindless" column is the one that applies here. See [Drawing cost](#bindless-is-off-for-sprites-here).

### Atlas frames and image swaps cost CPU, not batches

Changing the atlas index, image, flip or tint all change `Sprite`, and each one takes the same path. **Verified**, in
[`sprite_mesh/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_sprite_render/src/sprite_mesh/mod.rs):

1. `add_material` picks up `Changed<Sprite>`.
2. It builds a bucket key and scans that bucket comparing whole `Sprite` values. The atlas index isn't in the key, so
   all frames of one atlas share a bucket.
3. It queues one command that swaps the entity's `MeshMaterial2d`.

Further facts about that path:

- Each cached atlas material clones the whole `TextureAtlasLayout`. **Verified.**
- Materials are held weakly. A material no entity uses is removed, and `evict_unused_materials` walks the whole cache
  every frame. **Verified.**
- So a lone animated Pet probably creates and frees one material per animation frame, while a crowd on the same frames
  keeps them alive. **Inference**, not measured.
- [PR #25719](https://github.com/bevyengine/bevy/pull/25719) fixed sprites that change every frame never rendering. The
  fix is in the 0.20.1 source: `add_material` runs before `AssetEventSystems`, and `extract_2d_meshes` uses a placeholder
  binding. **Verified.**

### What each sprite costs per frame

**No published figure exists for 0.20.1.** This is where the work scales, read from the code. **Verified** for what
runs, with no timings:

| Work | Runs for | Source |
|---|---|---|
| Visibility check, on worker threads | Every sprite, every frame | [`check_visibility_cpu_culling`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_camera/src/visibility/mod.rs) |
| `extract_2d_meshes` | Only moved or changed sprites | `mesh2d/mesh.rs` |
| `add_material` and its command | Only sprites whose `Sprite` changed | `sprite_mesh/mod.rs` |
| Stable sort of `Transparent2d` | Every queued `Blend` sprite, every frame | [`core_2d/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_core_pipeline/src/core_2d/mod.rs) |
| Batch building: a hash lookup and one instance record | Every queued sprite, every frame | `batching/no_gpu_preprocessing.rs` |

A still sprite is cheap but not free. A moving sprite pays for extraction too. A sprite changing animation frame pays for the
material path as well.

### Published sprite counts

**None of these is for 0.20.1, and bevymark wasn't run here.** The
[0.20 release notes](https://bevy.org/news/bevy-0-20/) say performance improved in many cases but give no figures, and
[#25432](https://github.com/bevyengine/bevy/pull/25432) has none either.

| Source | Version | Path | Hardware | Sprites | Result | Per sprite, per frame |
|---|---|---|---|---|---|---|
| [#23039](https://github.com/bevyengine/bevy/issues/23039) | Dev build before 0.19 | **`Mesh2d`, closest to 0.20** | Ryzen 7950X, RTX 4070 Ti, Windows | 30,000 (`--waves 60 --per-wave 500`) | 13.7 ms, then 15.3 ms | 0.46 to 0.51 µs |
| [#17041](https://github.com/bevyengine/bevy/pull/17041) | 0.15.3 and a later dev build | Old sprite | Ryzen 7 9700X, RTX 3070, Linux | 100,000 (`--waves 100 --per-wave 1000`) | 17.53 ms, then 11.47 ms | 0.18, then 0.11 µs |
| [#3060](https://github.com/bevyengine/bevy/pull/3060) | 2021, pre-0.6 | Old sprite | Not stated | About 130,000 at 60 fps, 100,000 tinted | About 16.7 ms | About 0.13 to 0.17 µs |
| [#9597](https://github.com/bevyengine/bevy/pull/9597) | Pre-0.20 | Old sprite | Apple M1 Max | Not in text | 37% lower median frame time | No absolute number |
| [#17078](https://github.com/bevyengine/bevy/pull/17078) | After 0.15, [later reverted](https://github.com/bevyengine/bevy/pull/17123) | Old sprite | Apple M4 Max | 100,000 | 20.2% lower median frame time | No absolute number |

- The per-sprite column is my division. It's whole frame time, including moving the birds, the render thread and any
  GPU wait. It's wall time, so it can't be read as a share of one core.
- Bevy runs across many cores. I found no published CPU percentage for any of these.
- Bird counts come from multiplying the flags. #23039 is still open with status unclear.

The nearest comparison: about 0.5 µs per entity per frame to draw, against about 0.06 µs per actor per tick to
simulate. Rough, because the hardware and version differ.

To get a real number on this machine, from a `v0.20.1` Bevy checkout:

```text
cargo run --profile stress-test --example bevymark -- --benchmark --waves 30 --per-wave 1000
```

## 5. Utility AI prior art

**These sources don't depend on a Bevy version.** Primary sources read: Mike Lewis's
[chapter 13 of Game AI Pro 3](https://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter13_Choosing_Effective_Utility-Based_Considerations.pdf),
Kevin Dill's [Dual-Utility Reasoning](https://www.gameaipro.com/GameAIPro2/GameAIPro2_Chapter03_Dual-Utility_Reasoning.pdf)
and [Modular AI](https://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter08_Modular_AI.pdf), and the source of
[Curvature](https://github.com/apoch/curvature), an editor its README bases on Dave Mark's Infinite Axis Utility System.

### Infinite Axis shape

As used in Guild Wars 2: Heart of Thorns (Lewis), with field names from Curvature:

```text
Decision (Lewis calls it a DSE, Curvature a Behavior)
    action           what to do if this wins
    weight           a multiplier on the final score
    can_target_self, can_target_others
    considerations   any number, the "infinite axes"

Consideration
    input            which value to read, from self, target, or computed
    bookends         the range that maps to 0..1, anything outside is clamped
    curve            type, slope, exponent, x_shift, y_shift

Curve types in Curvature: linear, polynomial, logistic, logit, normal, sine
```

To score one decision:

1. Read each input and normalise it to 0..1 using the bookends.
2. Pass it through its curve, then clamp to 0..1.
3. Multiply all the scores together.
4. Stop as soon as a score hits 0. Any one consideration can veto a decision, and putting cheap ones first saves work.
5. The highest-scoring decision wins. Lewis strongly recommends always taking the best.

Other points worth keeping:

- Multiplying many values below 1 drags the score down as considerations are added. Curvature corrects each score `s`,
  with `n` considerations, as `s + s(1 - s)(1 - 1/n)`.
- Lewis says most curves in the shipped game came from a small set of presets, with free editing hidden away.
- Oscillation between two close decisions is a known problem. Lewis lists a small bonus for the last choice, per-decision
  weights, and runtime or cooldown considerations, and says they often only move the problem.
- A decision is usually scored per target, which multiplies the work. Inference from Curvature's `CanTargetOthers`
  and its `PropertyOfTarget` inputs.

### Dill's dual-utility shape

```text
Option
    considerations
    action

Consideration returns three numbers
    addend, multiplier, rank

Option weight = (sum of addends) x (product of multipliers)
Option rank   = highest rank of its considerations
```

To choose, per Dill:

1. Drop options with weight 0 or less.
2. Keep only the highest rank.
3. Optionally drop options far below the best weight.
4. Pick from the rest by weighted random.

Rank is what makes "must happen" behaviour simple: Zoo Tycoon 2 gave dying a rank of 1,000,000. Dill's GAIA system stores
all of this as XML configuration. Weighted random gives variety, which the Infinite Axis "always take the best" rule
doesn't.

### `big-brain`

| Fact | Status |
|---|---|
| Latest on [crates.io](https://crates.io/crates/big-brain): 0.22.0, published 2024-11-30. | Verified 2026-11-02 |
| The [GitHub repo](https://github.com/zkat/big-brain) was archived on 2025-10-07 and moved to [Codeberg](https://codeberg.org/zkat/big-brain). | Verified 2026-11-02 |
| Codeberg's main branch has version 0.23.0 depending on Bevy 0.17.0, with its README still saying 0.16. The latest commit I could see is from 2025-10-07. | Verified 2026-11-02 |
| **Not compatible with Bevy 0.20.** No release or main branch targets it. | Inference from the above |
| **Not maintained for current Bevy.** No commits visible for 13 months, and the newest target is 0.17, three releases behind. | Inference from the above |

## 6. Schedules in 0.20.1

### Disabled systems

| Fact | Status | Source |
|---|---|---|
| A condition is itself a small system. A false one still runs once per schedule run, and fetches its own parameters. | Verified | [`single_threaded.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/executor/single_threaded.rs), [`multi_threaded.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/executor/multi_threaded.rs) |
| The system itself doesn't run, and its query isn't touched. | Verified | Same |
| A set's condition is evaluated once per run. If false, every system in the set is marked skipped in one step. A system's own conditions are still evaluated. | Verified | Same |
| The multi-threaded executor spawns a task for each system that runs, and none for a skipped one. | Verified | `spawn_system_tasks` in `multi_threaded.rs` |
| `in_state(s)` is one such condition: it reads `Option<Res<State<S>>>` and compares. | Verified | [`condition.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_state/src/condition.rs) |
| `States` live in `bevy_state`, not `bevy_ecs`, and need `StatesPlugin`. `DefaultPlugins` includes it behind the `bevy_state` feature, which the project's `default_app` turns on. `MinimalPlugins` doesn't. | Verified | [`app.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_state/src/app.rs), [`default_plugins.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_internal/src/default_plugins.rs) |
| `StateTransition` runs each frame after `PreUpdate`. `OnEnter` and `OnExit` schedules run only on a transition. | Verified | [`transitions.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_state/src/state/transitions.rs) |
| `DespawnOnExit(state)` despawns an entity when its state ends. | Verified | [`state_scoped.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_state/src/state_scoped.rs) |
| A skipped system doesn't read messages. If it stays off for more than two updates, what was written meanwhile is gone by the time it returns. | Inference, from question 2 | |

The measured cost is in [Idle overhead](#idle-overhead).

### Choosing an executor

**`ExecutorKind` doesn't exist in 0.20.1.** It isn't in the source or on
[docs.rs](https://docs.rs/bevy_ecs/0.20.1/bevy_ecs/schedule/struct.Schedule.html). **Verified.**

| Fact | Status | Source |
|---|---|---|
| The API is `Schedule::set_executor(impl SystemExecutor)`, with `SingleThreadedExecutor` or `MultiThreadedExecutor`. So yes, it's per schedule. | Verified | [`schedule.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/schedule.rs) |
| Setting it re-initialises the executor on the next run. | Verified | Same |
| At setup, reach a schedule with `App::edit_schedule`. | Verified | [`app.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/app.rs) |
| `Main`, `FixedMain` and `RunFixedMainLoop` are already single-threaded. `Update` and `FixedUpdate` take the default, which is multi-threaded when the feature is on. | Verified | [`main_schedule.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/main_schedule.rs), [`executor/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/executor/mod.rs) |
| Switching executor while the app runs should work from an exclusive system on a schedule that isn't running. A running schedule is removed from `Schedules`. | Inference, not run | `schedule_scope` in `world/mod.rs` |

### Parallel iteration setup

| Fact | Status | Source |
|---|---|---|
| `par_iter_mut` needs the `multi_threaded` feature. Without it, it quietly runs on one thread. | Verified | [`query/par_iter.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/query/par_iter.rs) |
| It needs `ComputeTaskPool` to exist, and panics otherwise. `ComputeTaskPool::get_or_init` with a `TaskPoolBuilder` sets it up. | Verified, and used in the benchmark | [`usages.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_tasks/src/usages.rs) |
| The multi-threaded executor creates a default pool itself. The single-threaded one doesn't. | Verified | `multi_threaded.rs` |
| In an `App`, `TaskPoolPlugin` makes the pools: IO and async compute each get 25% of cores, at most 4, and compute gets the rest. That's 24 compute threads on 32 logical processors. | Verified, the 24 is Inference | [`task_pool_plugin.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/task_pool_plugin.rs) |
| Default batch size is the largest table divided by thread count. `batching_strategy` can set a minimum, which would keep small crowds on fewer threads. | Verified, effect not measured | [`batching.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/batching.rs) |
| Iteration order and thread assignment aren't guaranteed. | Verified | [`system/query.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/system/query.rs) |
| The default strategy assumes each entity has roughly the same work. | Verified | `batching.rs` |
| So one expensive boss would make its batch the slowest thread in that `par_iter_mut`. A boss with extra components would sit in its own table, and so run as its own task. | Inference | |

### Making parallel schedules deterministic

| Fact | Status | Source |
|---|---|---|
| `.chain()`, `.before()` and `.after()`, on systems or sets, are hard ordering rules. | Verified | [`config.rs`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_ecs/src/schedule/config.rs) |
| Without an ordering rule, the run order of two conflicting systems is arbitrary, and an unrelated change can flip it. | Verified | [Release note](https://github.com/bevyengine/bevy/blob/v0.20.1/_release-content/release-notes/schedule_randomization.md) |
| `ScheduleBuildSettings::ambiguity_detection` reports conflicting pairs with no order. It defaults to `LogLevel::Ignore`. `Warn` logs them, `Error` also fails. | Verified | `schedule.rs` |
| `ambiguous_with` and `ambiguous_with_all` mark a conflict as accepted. | Verified | `config.rs` |
| New in 0.20: `chain_weak`, `before_weak` and `after_weak` keep order only between systems whose data really conflicts, so the rest can overlap. They can't see hidden dependencies such as interior mutability. | Verified | [Release note](https://github.com/bevyengine/bevy/blob/v0.20.1/_release-content/release-notes/chain_weak.md) |
| New in 0.20: `ScheduleBuildSettings::shuffle_seed`, behind the `debug` feature, randomises system order within the rules, to catch missing ones. The notes suggest the single-threaded executor for it. | Verified | `schedule_randomization.md` |
| `auto_insert_apply_deferred` is on by default, adding a sync point so a later ordered system sees earlier `Commands`. | Verified | `schedule.rs` |
| Schedule ordering says nothing about order inside one `par_iter_mut`. The result is only deterministic if each entity's work doesn't depend on the others. Here it doesn't, and all variants ended identical. | Measured | [Benchmark](#benchmark) |

### Headless `App` for tests

Yes. **Verified.**

- [`MinimalPlugins`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_internal/src/default_plugins.rs) is
  `TaskPoolPlugin`, `FrameCountPlugin`, `TimePlugin` and `ScheduleRunnerPlugin`. There's no window or renderer.
- [`App::update`](https://github.com/bevyengine/bevy/blob/v0.20.1/crates/bevy_app/src/app.rs) runs one pass by hand, so
  the runner never starts. `TimePlugin` brings the fixed loop, so `FixedUpdate` runs here too.
- The project already does this: the animation test in
  [`placeholder.rs`](../../crates/pet-app/src/engine/placeholder.rs) uses `MinimalPlugins`, `ManualDuration` and
  repeated `app.update()`.
- `TimeUpdateStrategy::FixedTimesteps(n)` runs exactly n fixed steps per update.
- Tests that use `States` must add `StatesPlugin` themselves.

## Benchmark

**Setup.** Each variant calls the same per-actor functions. One crate, `bevy_ecs =0.20.1` and `bevy_tasks` with
`multi_threaded`, `--release` with the workspace's `codegen-units = 1` and thin LTO. Ryzen 9 3950X, 32 logical
processors, rustc 1.97.1. The parallel variants use 8 threads. Figures are the median of three runs. The machine was 10
to 17% busy, so treat gaps under about 10% as noise. Code and raw logs are in
`C:\Users\flynn\AppData\Local\Temp\desktop-pet-sim-compare`, outside the repo.

**One tick:** motion, clamp, build the enemy lookup per team, then per actor a nearest-enemy lookup and a six-state
machine. Two teams start apart. Each N runs a 90-tick warm-up and the same fixed tick count in every variant (2,000,
1,000, 150, 30).

**Proof of equal work.** After each variant the end state is hashed, and the run stops if it differs from single-threaded
plain Rust. All variants matched, in every run. Lookups passed a brute-force check at N = 2,000.

**In the tables,** the first figure is the wall time of a tick. The percentage is total CPU across all threads as a
share of one core at 30 ticks a second, read from the process cycle counter. A tick must finish inside 33,333 µs. On
the default 64 Hz fixed step, costs are 2.13 times these (Inference, question 3).

### Walkers on one line, sorted array

| Variant | N = 200 | 1,000 | 10,000 | 100,000 |
|---|---|---|---|---|
| Plain, 1 thread | 5.2 µs, 0.016% | 36 µs, 0.11% | 503 µs, 1.6% | 6,058 µs, 18% |
| ECS one component, `iter_mut` | 7.4 µs, 0.024% | 59 µs, 0.19% | 701 µs, 2.2% | 8,045 µs, 24% |
| ECS one component, change detection bypassed | 6.0 µs, 0.019% | 40 µs, 0.13% | 585 µs, 1.9% | 6,298 µs, 19% |
| ECS four components, `iter_mut` | 6.3 µs, 0.019% | 42 µs, 0.13% | 623 µs, 2.0% | 6,443 µs, 20% |
| Plain, `thread::scope` | 999 µs, 6.6% | 1,172 µs, 8.2% | 1,368 µs, 10% | 3,045 µs, 31% |
| Plain, `bevy_tasks` pool | 23 µs, 0.17% | 69 µs, 0.46% | 313 µs, 2.6% | 2,621 µs, 26% |
| ECS one component, `par_iter_mut` | 59 µs, 0.39% | 106 µs, 0.69% | 399 µs, 3.2% | 2,687 µs, 29% |
| ECS four components, `par_iter_mut` | 66 µs, 0.43% | 126 µs, 0.86% | 458 µs, 4.3% | 3,073 µs, 37% |

An earlier plain single-thread run split the 100,000 tick into motion 0.14 ms, sorting 2.2 to 2.3 ms and decisions 3.6
to 3.7 ms.

### Flyers in 2D, uniform grid

| Variant | N = 200 | 1,000 | 10,000 | 100,000 |
|---|---|---|---|---|
| Plain, 1 thread | 12 µs, 0.041% | 83 µs, 0.27% | 799 µs, 2.4% | 15,568 µs, 48% |
| ECS one component, `iter_mut` | 13 µs, 0.043% | 86 µs, 0.26% | 786 µs, 2.4% | 15,161 µs, 47% |
| ECS one component, change detection bypassed | 13 µs, 0.043% | 85 µs, 0.27% | 797 µs, 2.5% | 14,921 µs, 46% |
| ECS four components, `iter_mut` | 13 µs, 0.044% | 91 µs, 0.29% | 799 µs, 2.5% | 16,697 µs, 52% |
| Plain, `thread::scope` | 1,169 µs, 8.1% | 1,211 µs, 8.5% | 1,269 µs, 11% | 5,447 µs, 80% |
| Plain, `bevy_tasks` pool | 37 µs, 0.28% | 95 µs, 0.73% | 378 µs, 4.3% | 4,573 µs, 75% |
| ECS one component, `par_iter_mut` | 86 µs, 0.54% | 144 µs, 1.0% | 453 µs, 5.0% | 4,697 µs, 78% |
| ECS four components, `par_iter_mut` | 90 µs, 0.56% | 144 µs, 1.1% | 442 µs, 5.1% | 4,672 µs, 79% |

The grid is rebuilt every tick by counting sort, with cells sized for about two enemies each. A flyer's sight is 400 px.

**16 threads, one run, 100,000 actors, wall time:**

| Variant | Walkers | Flyers |
|---|---|---|
| Plain, `bevy_tasks` pool | 2,395 µs | 3,243 µs |
| ECS one component, `par_iter_mut` | 2,474 µs | 3,272 µs |
| ECS four components, `par_iter_mut` | 2,655 µs | 3,290 µs |

Doubling threads helps flyers more than walkers. Inference: the walkers' sort only ever splits in two, and building the
lookup's input is single-threaded in every variant.

### The mirror

Copying position and action id from `Vec<Actor>` into N entities, on its own. Run in a one-system `Schedule`, one copy
per tick, with the simulation running untimed between copies. The target is a component with the same ten floats as
`Transform`, plus a `u16`. The percentage is wall time as a share of one core at 30 Hz.

| Method | N = 200 | 1,000 | 10,000 | 100,000 |
|---|---|---|---|---|
| `iter_mut().zip()`, entity order matches the `Vec` | 0.4 µs, 0.001% | 1.5 µs, 0.004% | 14 µs, 0.043% | 405 µs, 1.2% |
| `get_mut(entity)` per actor | 1.4 µs, 0.004% | 6.6 µs, 0.020% | 65 µs, 0.20% | 795 µs, 2.4% |
| `par_iter_mut` with an index component, wall only | 33 µs, 0.098% | 34 µs, 0.10% | 68 µs, 0.20% | 406 µs, 1.2% |

This is the copy alone. It leaves out spawning, despawning and everything the render side then does with the data. The
`zip` row only works while entity order matches the `Vec`, and a despawn on either side breaks that, so `get_mut` is the
robust figure. Inference.

### Idle overhead

A `bevy_ecs` schedule of 30 systems over six components, in a world with one unrelated entity, run at a real 30 Hz for
3 s. Three runs, two with a pool of 8 and one with 24. "Above the empty loop" compares whole-process CPU, as a share of
one core, against the same 30 Hz loop with no schedule, which itself used about 0.07%. The noise is about ±0.03%, so
negative values just mean nothing could be seen.

| Executor | 30 systems | Median schedule run | CPU above the empty loop |
|---|---|---|---|
| Single | Each with its own false `run_if` | 8.5 to 9.8 µs | 0.007 to 0.018% |
| Single | In one set with one false `run_if` | 1.7 to 8.6 µs | -0.015 to 0.029% |
| Single | No condition, queries match nothing | 3.0 to 12.4 µs | -0.006 to 0.028% |
| Multi | Each with its own false `run_if` | 12.5 to 15.0 µs | 0.016 to 0.040% |
| Multi | In one set with one false `run_if` | 3.0 to 11.0 µs | -0.016 to 0.025% |
| Multi | No condition, queries match nothing | 68 to 89 µs | 0.46 to 0.59% |

- Only the last row rises above the noise. Those systems all run, so each one spawns a task.
- Pool size made no visible difference between 8 and 24 threads.
- Run times swing a lot. Inference: the caches are cold after each 33 ms sleep.
- The conditions here read a `Res<bool>`, the same shape as `in_state`.

### Where the comparison isn't fair

- **`thread::scope` against `par_iter_mut`: unfair.** An empty `thread::scope` with 7 threads costs 465 µs on Windows,
  because it creates OS threads each time, and this tick uses three scopes. Bevy's pool stays alive. The `bevy_tasks` pool
  row is the fair one. It's plain data with no ECS, but it isn't zero crates.
- **Plain against ECS one component: fair on layout.** Same struct, same field order, same code. The ECS adds change
  detection and schedule overhead.
- **Plain against ECS four components: a different layout.** Each phase there touches only the fields it needs. It's the
  shape real ECS code would have.
- **Plain Rust was only tried as an array of structs.** A struct-of-arrays version wasn't tested and might beat both.
- **The mirror isn't a cost only plain Rust pays.** Both ECS layouts keep position in their own component, not in
  `Transform`, so sprites would need a copy there too, unless the simulation writes `Transform` directly. Inference.
- **8 threads here, 24 in a default `App`.** On this machine an `App` would split 200 actors into batches of about 9,
  which is likely worse than the N = 200 rows show. Inference.
- **One-component ECS is slower than four components on the line,** at 8,045 against 6,443 µs. Bypassing change
  detection removes the gap, so change-tick writes are the cause there. Why four components with three writes don't pay
  the same isn't explained.
- **The ECS `par_iter_mut` rows use the multi-threaded executor on three chained systems,** so nothing overlaps between
  systems. All the speed-up is inside each system.
- **Nothing here spawns or despawns.** Every entity shares one table. Waves dying, mixed walkers and flyers, and bosses
  with extra components would add table moves and more tables in the ECS, and swap-removal in plain Rust. Not measured.
- **The grid is tuned to the test.** Its first version took 1,136 ns per actor at 100,000, because actors with nothing
  in sight walked every ring. A quick empty-box test brought that to 156 ns. All actors flying is also the worst case.
- **State mix.** N = 200 settles into a fight, about 65% attacking. N = 1,000 is about 45%. N = 10,000 and 100,000 stay
  mostly idle and walking, because they run far fewer ticks. Compare down a column, not across a row.
- **These are floors.** No utility scoring, collision, animation or messages. Every actor decides every tick. A smarter
  boss would add to the decision time, and bigger actors would cost more in every variant.

### Against the budget

The owner's new line is that companion mode stays very light and encounters may use much more, across all cores.
[PRD §6.2](../PRD.md) doesn't say this yet. Its row of under 1% of the machine, about 32% of one core on 32 logical
processors, is written for "idle/animating". The spike uses about 15% of a core. Inference.

## Drawing cost, measured

**Measured 2026-11-03**, Bevy 0.20.1, Ryzen 9 3950X with 32 logical processors, RTX 2080 Ti on driver 32.0.16.1088,
3840×2160 screen. Code, the CSV and the run log are in `C:\Users\flynn\AppData\Local\Temp\desktop-pet-sprite-cost`,
outside the repo.

**Setup.** This is a throwaway stress app, not bevymark, because bevymark can't vary sprite size.

- It uses the workspace's Bevy features and release profile, and the repo's `Cargo.lock`.
- It copies the app's render setup:
  - DX12, the Visual swapchain, `PreMultiplied` alpha, `ClearColor(NONE)` and `Msaa::Off`.
  - `MemoryHints::MemoryUsage`, with `PipelinedRenderingPlugin` disabled.
  - `reactive_low_power` at 1/30 s, and `AutoVsync`.
  - A transparent, always-on-top window covering the work area, 3840×2088, which never activates and lets clicks
    through.
- Each sprite has an 8-frame atlas, 4×2, made in code. A frame is half the on-screen size and drawn with a nearest
  sampler, like the Pet at scale 2. So it's 64 px for 128 on screen, and 256 px for 512.
- Sprites are placed at random inside the window, each with its own z. Texture and starting frame are random, so
  neighbours in draw order differ.
- **Moving:** every sprite moves, bounces off the window edges, and changes atlas frame every frame.
- **Still:** nothing changes after spawning.
- **20 textures** are 20 atlases in different colours, standing in for enemy types.
- After the 4 s warm-up, the app counted sprites with `ViewVisibility` true. It was N in every run, so nothing was
  culled.

Each combination ran in its own process: a 4 s warm-up, then 20 s measured.

**What was measured, and how.**

| Figure | How |
|---|---|
| CPU | `GetProcessTimes` on the process from outside, as in [`metrics.rs`](../../crates/pet-app/tests/e2e/metrics.rs). Share of the machine is the one-core share ÷ 32. |
| GPU | The same counter as `metrics.rs`, `\GPU Engine(pid_N_*)\Utilization Percentage`, summed over engines and averaged over the 20 s. |
| Memory | Private working set (Task Manager's "Memory") at the end of the 20 s. |
| Frame interval | Time between the starts of two updates, read inside the app. |
| Work time | From the start of an update to the end of the render world's frame, read inside the app. That includes any wait on the GPU. It's the figure that shows headroom, because the 30 fps cap makes every frame interval read 33.3 ms. |

**Every run held 30 fps.** Frame interval p95 was 33.7 to 34.0 ms in all 48 runs. **Measured.**

The fixed cost, an empty overlay with no sprites: work time p50 2.2 ms, 13.1% of one core (0.41% of the machine), GPU
2.6%, 85.0 MB. **Measured.** The spike's figure was about 15% of one core.

In the tables, **work** is the p50 / p95 work time in ms, with 33.3 ms the limit. **Core** is CPU as a share of one
core, and **machine** as a share of all 32. All figures are **Measured**, one run each.

### 128×128

| Count | Motion | Textures | Work p50 / p95 | Core | Machine | GPU | Memory |
|---|---|---|---|---|---|---|---|
| 200 | Still | 1 | 3.2 / 3.9 | 17% | 0.55% | 3.3% | 91 MB |
| 200 | Still | 20 | 2.8 / 3.6 | 14% | 0.44% | 3.2% | 112 MB |
| 200 | Moving | 1 | 2.7 / 3.4 | 15% | 0.47% | 3.9% | 93 MB |
| 200 | Moving | 20 | 4.4 / 6.1 | 23% | 0.72% | 2.3% | 117 MB |
| 1,000 | Still | 1 | 3.6 / 4.5 | 18% | 0.57% | 5.7% | 94 MB |
| 1,000 | Still | 20 | 3.2 / 3.9 | 16% | 0.49% | 9.1% | 117 MB |
| 1,000 | Moving | 1 | 3.8 / 4.5 | 20% | 0.61% | 4.4% | 95 MB |
| 1,000 | Moving | 20 | 4.2 / 5.0 | 22% | 0.69% | 3.7% | 121 MB |
| 5,000 | Still | 1 | 5.9 / 7.1 | 30% | 0.92% | 9.9% | 121 MB |
| 5,000 | Still | 20 | 6.7 / 8.0 | 32% | 1.01% | 12.2% | 136 MB |
| 5,000 | Moving | 1 | 10.8 / 12.6 | 45% | 1.40% | 10.9% | 127 MB |
| 5,000 | Moving | 20 | 11.4 / 13.0 | 48% | 1.50% | 13.1% | 139 MB |
| 10,000 | Still | 1 | 10.1 / 11.5 | 44% | 1.36% | 14.5% | 143 MB |
| 10,000 | Still | 20 | 11.5 / 13.1 | 43% | 1.35% | 14.6% | 158 MB |
| 10,000 | Moving | 1 | 19.1 / 21.8 | 70% | 2.19% | 12.8% | 147 MB |
| 10,000 | Moving | 20 | 21.2 / 24.2 | 75% | 2.33% | 18.3% | 167 MB |

### 256×256

| Count | Motion | Textures | Work p50 / p95 | Core | Machine | GPU | Memory |
|---|---|---|---|---|---|---|---|
| 200 | Still | 1 | 2.5 / 3.2 | 15% | 0.46% | 4.9% | 98 MB |
| 200 | Still | 20 | 2.5 / 3.3 | 15% | 0.45% | 3.7% | 148 MB |
| 200 | Moving | 1 | 2.7 / 3.3 | 15% | 0.46% | 4.2% | 100 MB |
| 200 | Moving | 20 | 5.1 / 6.2 | 21% | 0.67% | 5.8% | 147 MB |
| 1,000 | Still | 1 | 3.4 / 4.2 | 19% | 0.60% | 5.2% | 102 MB |
| 1,000 | Still | 20 | 3.5 / 4.2 | 17% | 0.53% | 11.8% | 145 MB |
| 1,000 | Moving | 1 | 4.2 / 5.0 | 19% | 0.59% | 8.8% | 105 MB |
| 1,000 | Moving | 20 | 4.7 / 5.5 | 26% | 0.80% | 11.7% | 148 MB |
| 5,000 | Still | 1 | 6.6 / 7.7 | 28% | 0.86% | 17.5% | 124 MB |
| 5,000 | Still | 20 | 6.5 / 7.5 | 28% | 0.89% | 14.8% | 172 MB |
| 5,000 | Moving | 1 | 10.7 / 12.2 | 42% | 1.32% | 17.5% | 124 MB |
| 5,000 | Moving | 20 | 11.4 / 12.9 | 46% | 1.44% | 14.9% | 180 MB |
| 10,000 | Still | 1 | 10.1 / 11.9 | 44% | 1.37% | 25.1% | 140 MB |
| 10,000 | Still | 20 | 10.8 / 12.5 | 41% | 1.28% | 28.6% | 185 MB |
| 10,000 | Moving | 1 | 19.7 / 23.8 | 73% | 2.29% | 24.1% | 143 MB |
| 10,000 | Moving | 20 | 22.2 / 27.1 | 83% | 2.60% | 24.8% | 193 MB |

### 512×512

| Count | Motion | Textures | Work p50 / p95 | Core | Machine | GPU | Memory |
|---|---|---|---|---|---|---|---|
| 200 | Still | 1 | 2.9 / 3.7 | 17% | 0.54% | 5.8% | 94 MB |
| 200 | Still | 20 | 2.6 / 3.2 | 16% | 0.49% | 10.1% | 120 MB, peak 146 |
| 200 | Moving | 1 | 2.6 / 3.3 | 14% | 0.42% | 9.4% | 96 MB |
| 200 | Moving | 20 | 4.4 / 5.3 | 20% | 0.61% | 10.7% | 158 MB, peak 182 |
| 1,000 | Still | 1 | 3.2 / 3.9 | 17% | 0.54% | 14.2% | 97 MB |
| 1,000 | Still | 20 | 3.2 / 3.9 | 18% | 0.56% | 10.0% | 168 MB, peak 182 |
| 1,000 | Moving | 1 | 4.2 / 4.9 | 21% | 0.66% | 12.8% | 103 MB |
| 1,000 | Moving | 20 | 4.4 / 5.2 | 21% | 0.65% | 15.7% | 173 MB, peak 185 |
| 5,000 | Still | 1 | 5.9 / 6.7 | 28% | 0.86% | 24.7% | 118 MB |
| 5,000 | Still | 20 | 7.0 / 8.1 | 27% | 0.85% | 27.6% | 189 MB, peak 202 |
| 5,000 | Moving | 1 | 10.8 / 12.1 | 46% | 1.42% | 21.0% | 123 MB |
| 5,000 | Moving | 20 | 11.6 / 12.9 | 46% | 1.45% | 27.9% | 191 MB, peak 205 |
| 10,000 | Still | 1 | 9.8 / 11.4 | 42% | 1.32% | 34.2% | 138 MB |
| 10,000 | Still | 20 | 11.0 / 13.7 | 47% | 1.47% | 50.3% | 209 MB, peak 222 |
| 10,000 | Moving | 1 | 19.7 / 22.6 | 72% | 2.26% | 34.1% | 145 MB |
| 10,000 | Moving | 20 | 20.8 / 23.6 | 74% | 2.32% | 50.2% | 214 MB, peak 227 |

### What the tables show

- **Count sets the CPU cost, and size doesn't.** At 10,000 moving, work time is 19 to 22 ms at every size. **Measured.**
- **Moving and animating doubles the cost of a still crowd** from 5,000 up: 10 to 11.5 ms still, 19 to 22 ms moving.
  **Measured.**
- **20 textures instead of 1 adds little CPU**, 0 to 2.5 ms at 10,000. Its real cost is memory. **Measured.**
- **Per sprite, per frame:** about (21 − 2.2) ms ÷ 10,000 ≈ 1.9 µs moving and animated, and about 0.8 to 0.9 µs still.
  **Inference**, from the slope between 0 and 10,000. The nearest published figure, about 0.5 µs, was for still sprites
  on faster hardware ([Published sprite counts](#published-sprite-counts)).
- **The work is in the render world, not the game update.** At 10,000 moving, the main schedule takes 2.9 to 3.4 ms
  p50, including the motion system. The other 16 to 19 ms is extraction, the material swap, sorting, batching and draw
  recording, plus any wait for the GPU. **Measured** for the split. Which of those dominates wasn't profiled.
- **The CPU limit is one thread.** With pipelined rendering off, the update and the render run one after the other on
  the main thread. 72 to 83% of one core, against 20 to 23 ms of a 33 ms frame, fits one busy thread plus some worker
  help. **Inference.**
- **The GPU isn't the limit up to 10,000.** Its highest reading was 50% at 512×512 × 10,000 with 20 textures, at full
  clock, 1,920 MHz. That case covers the screen about 330 times over (10,000 × 512² ÷ 3840 × 2088 pixels), a fill-rate
  worst case no real encounter reaches. **Measured**, and the overdraw figure is arithmetic.

**How many fit in 33 ms.** One row per sprite size:

| Size | Measured | Inference |
|---|---|---|
| 128×128 | At least 10,000 moving, at p95 24 ms | About 16,000 moving at p50, or 13,000 to 14,000 to keep p95 under 33 ms |
| 256×256 | At least 10,000 moving, at p95 27 ms | About the same as 128 at p50. Somewhat fewer to keep p95 under 33 ms, because this p95 was the highest |
| 512×512 | At least 10,000 moving, at p95 24 ms | About the same as 128 on CPU. The GPU was at 50% at full clock, so it would run out somewhere past 20,000 |

- **Single frames already come close to the limit at 10,000 moving.** The slowest frame in those runs took 27 to 33
  ms of work. One frame at 256 × 5,000 took 34 ms. Frame interval p95 stayed under 34 ms. **Measured.**
- **In budget terms**, 200 moving enemies cost 14 to 23% of one core (0.4 to 0.7% of the machine). That's about the
  same as the empty overlay's 13%. 10,000 moving cost 2.2 to 2.6% of the machine. **Measured.**

### Memory

| Case | Private working set | Status |
|---|---|---|
| Empty overlay | 85 MB | Measured |
| 1 texture, 200 to 10,000 sprites | 91 to 147 MB, rising about 5 to 6 MB per 1,000 sprites | Measured |
| 20 textures, 200 sprites | 112 MB at 128, 147 to 148 MB at 256, 120 to 158 MB at 512 | Measured |
| 20 textures, 10,000 sprites | 158 to 167 MB at 128, 185 to 193 MB at 256, 209 to 227 MB at 512 | Measured |

- The PRD's 150 MB line is for companion mode. Twenty 512-sized enemy types pass it even at 200 sprites.
- The 20 textures themselves are 2.5 MB of pixels at 128 and 40 MB at 512, with the CPU copy dropped
  (`RENDER_WORLD`). So at 512 most of the jump is texture bytes. At 128 the jump of 12 to 26 MB is more than the
  pixels, probably the 160 distinct materials, each with its own bind group, against 8. **Inference.**

### Bindless is off for sprites here

| Fact | Status | Source |
|---|---|---|
| In the running app, `material_uses_bindless_resources::<SpriteMeshMaterial>` returns false. `SpriteMeshMaterial::bindless_supported` is false, while `bindless_slot_count` is 2048. | Measured | Printed from the render world |
| The device has `TEXTURE_BINDING_ARRAY`, 262,144 storage buffers per stage and 2048 binding-array samplers. | Measured | Same |
| The `AsBindGroup` macro adds 1 to its sampler count for each `#[texture]` as well as each `#[sampler]`. The sprite material has one of each, so it asks for 2 × 2048 = 4096 samplers, which this GPU doesn't have. | Verified | `bevy_render_macros-0.20.1/src/as_bind_group.rs`, lines 643, 769 and 990 to 999 |
| So each distinct `Sprite` material has its own bind group. Neighbours with a different texture or atlas frame split a batch, which is the "Without bindless" column in [question 4](#how-batching-works-now). | Inference | From the rows above |
| In a short smoke test, 2,000 still sprites all on frame 0 took 2.3 ms of work against 3.6 ms with random frames. | Measured, one 2 s run each | Same app, `sync` mode |

Counting the texture as a sampler looks like a Bevy bug. Whether a later Bevy fixes it wasn't checked.

### Where the comparison isn't fair

- **Background load.** Other processes used 5 to 10% of the machine throughout, about 2 to 3 cores. The owner was using
  the machine, and the cursor moved during the early runs. The stress app never moved it, and never became the
  foreground window.
- **GPU clock changed between runs.** One `nvidia-smi` reading mid-run ranged from P8 at 405 MHz to P0 at 1,920 MHz.
  The driver kept the light runs at a low clock, so their GPU percentages overstate the load against a GPU at full
  speed. Compare GPU figures only between runs at a similar clock. The 512 × 10,000 runs were at 1,710 to 1,920 MHz.
- **One run per combination.** Differences under about 1 ms of work, 3 points of CPU or a few MB look like noise. Some
  small rows are out of order, such as 128 × 200 still with 1 texture costing more than with 20.
- **The closing empty-overlay run was cut off** when the time budget ran out, so drift over the session wasn't checked.
- **Art is uniform.** All sprites are the same size, with about half their pixels transparent, and no tint, flip or
  text. Real art, particles and health bars would add draws.
- **No spawning, despawning or simulation.** The motion system is a trivial bounce. The simulation figures above add to
  these.
- **One machine, one GPU.** A GPU with bindless sprites, or a slower CPU, would change the picture. Inference.

## Not verified

- **Drawing past 10,000 sprites.** [Drawing cost](#drawing-cost-measured) measured up to 10,000. The counts that fit in
  33 ms beyond that are extrapolated. bevymark itself wasn't run.
- **Where the render-world time goes.** About 16 to 19 ms at 10,000 moving sprites wasn't broken down by system.
- **Drawing cost with real art,** tint, flip, `Text2d`, spawning and despawning, or a running simulation.
- **Drift over a session of stress runs.** The closing empty-overlay run was cut off.
- **Spawn, despawn and multi-table cost.** See [the fairness notes](#where-the-comparison-isnt-fair).
- **Idle cost inside a real `App`.** The idle test is a lone `bevy_ecs` schedule, not Bevy's full main loop.
- **Why change detection costs one component more than four.**
- **Switching executor at runtime.** Read from code, not run.
- **Bindless on other GPUs.** It's off here, [measured](#bindless-is-off-for-sprites-here). A GPU with at least 4096
  binding-array samplers would pass Bevy's check. None was tried.
- **Whether a later Bevy stops counting the sprite texture as a sampler.**
- **Material churn for a lone animated Pet.** Read from code, not run.
- **Message pile-up while `Time<Virtual>` is paused.** Read from code, not run.
- **Mark's own talks.** [Building a Better Centaur](http://www.gdcvault.com/play/1021848/Building-a-Better-Centaur-AI)
  wasn't read. The field names come from Lewis and Curvature. The `m`, `k`, `b`, `c` curve naming often attributed to him
  appeared in no primary source I read.
- **Whether a `big-brain` branch for 0.20 exists** outside Codeberg's main branch.
