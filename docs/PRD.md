# Desktop Pet — Product Requirements Document

| | |
|---|---|
| **Working title** | Desktop Pet (placeholder) |
| **Status** | Draft v0.1 — initial PRD from design interview |
| **Date** | 2026-10-29 |
| **Owner** | flynn |

---

## 1. Summary

A desktop companion that lives on top of your other Windows apps while you work. It walks along the
taskbar, behaves for its own reasons, reacts to you, and **grows and transforms visibly in its art** as
you progress together. The MVP is an ambient companion; after that, parallel tracks — pulled in whenever
desired, in no fixed order — add productivity features (focus timer, reminders), a light RPG where you and
the pet fight together, and an AI companion you can talk to.

The first character is a **dragon** that hatches from an egg the moment you first launch the app.

## 2. Product pillars

Every feature is judged against these. A feature that serves none of them is questioned; a feature that
violates one needs an explicit justification.

1. **Alive** — it behaves for reasons, and its reasoning is visible. When it experiments it shows it (❓), its
   experiments stay within limits and fade as it learns, and nothing it does goes unexplained.
2. **Never in the way** — click-through, auto-hide, no guilt mechanics, nothing attention-grabbing while
   you focus.
3. **Better together** — you and the pet are a team, in play and (later) in combat.
4. **Grows with you** — progression is *visible*: it hatches, grows, evolves and transforms in the art
   itself, never just in a number.

### 2.1 Design principles (2026-11-06)

How the pillars are kept as the Pet grows more capable:

- **Emergence.** Actions and mechanics are authored, and how they combine is learned. The Pet learns to combine what
  it can do through play with the owner and through its own Experiments, the way players find strategies the
  designers never planned ([ADR-0005](adr/0005-a-learning-mind-with-reasons.md)).
- **No scenario-specific Actions.** No Action or rule exists only to make one scenario work. New Actions are general
  building blocks, useful in their own right. Mechanics are general and compose: a smash's damage comes from the fall
  speed physics already gives, not from a special rule.
- **"Why" is a hard rule.** Every choice traces back to named causes: drives, memories, learned Links or an
  Experiment. This keeps deep neural networks out of the Mind for now. It's revisited if the Pet's intelligence falls
  short.
- **Packs are data, mechanisms are Rust.** A Pack names its Actions, drives, traits, Materials and Mind. The kinds of
  node, effect and rule they are built from are written in Rust, with no scripting.

## 3. Audience and platform

- **Audience:** personal use on the owner's own private devices. Decisions must not *preclude* a public
  release later (e.g. a real installer, a clean separation between app and art), but the MVP does no
  store, auto-update or distribution work.
- **Content:** no content-rating constraint for personal use. What art is bundled with any official
  distribution is a separate, later decision — made easy by the content-pack architecture (§7.3).
- **Platform:** Windows 11 only for the MVP. Reference machine: 3840×2160 monitor, NVIDIA RTX 2080 Ti.
  macOS/Linux are vision items.

## 4. Goals and non-goals

### MVP goals
- A pet that feels **alive** on the desktop: intelligent-looking behaviour, not a random-state screensaver.
- Never obstructs work: clicks pass through everything except the pet; it hides for fullscreen apps.
- One real character at the art bar (§6.1), proving the full growth pipeline in miniature
  (egg → hatch → hatchling).
- A foundation (behaviour engine, XP engine, form graph, content packs) that every later track builds on.

### MVP success bar
> "I'd leave it running all day and smile at it at least once."

Plus all acceptance criteria in §5 and the non-functional budget in §6.

### Non-goals (MVP)
- Tamagotchi-style needs that decay or punish neglect — **never**, not just not-in-MVP.
- Multi-monitor roaming, window-aware climbing, live 3D rendering, other operating systems.
- Productivity features, combat, AI chat (all post-MVP tracks).
- Cross-device save sync — **not planned**, including in the vision.
- Any form of keystroke capture (see §5.5).

## 5. MVP functional requirements

IDs are stable references for issues and tests.

**Where each requirement is delivered** (roadmap §9.1; item 4's slices are in
[item-4-architecture.md](proposals/item-4-architecture.md)):

| Delivered by | Requirements |
|---|---|
| Items 2 and 3 (built) | WIN-01–04, WIN-07–11, REN-01–05 (REN-04 draws Emotes; item 4 slice 7 decides when to show them) |
| Item 4, slice 1: Ground + throw | WIN-06, INT-01, WIN-04's exception while holding the Pet; BEH-06 fall, land, being-held |
| Item 4, slice 2: Save | SAV-01, SAV-02, SET-02 |
| Item 4, slice 3: Petting + hatching | INT-02, PRG-01, PRG-02, PRG-05 |
| Item 4, slice 4: Reactions | INT-04 (a single poke); BEH-06 react-to-poke, being-petted |
| Item 4, slice 5: XP + Level | PRG-03, PRG-04, PRG-06, INT-05 |
| Item 4, slice 6: Pet panel | Hosts SET-01 and the balancing panel of §8.2 |
| Item 4, slice 7: The Mind | BEH-01, BEH-02, BEH-05, BEH-07; BEH-06 idle, walk, sit, sleep |
| Item 4, slice 8: Senses + memory | BEH-03, BEH-04, INT-03 (moved from the old item 5), INT-04 (combos); BEH-06 look-at, follow/pounce |
| Item 4, slice 9: Settings + the rest | SET-01, SAV-03, AUD-01, WIN-05, rebinding for WIN-08, and PKG-01 |
| Every slice | SEN-01, SEN-02 |

### 5.1 Desktop presence

| ID | Requirement |
|---|---|
| WIN-01 | Borderless, per-pixel transparent window; only the sprite (and its effects) is visible. |
| WIN-02 | Always on top of normal application windows. |
| WIN-03 | Hidden from the taskbar **and** Alt-Tab. |
| WIN-04 | **Click-through everywhere except the pet's opaque pixels.** Clicks, drags and right-clicks on the pet go to the pet; everything else goes to the app underneath. **Exception while holding the Pet:** while the owner holds the Pet, the Overlay catches clicks wherever the cursor is, because the Pet trails behind the cursor. It stops when the hold ends or when Windows takes the mouse away, as Alt-Tab does. |
| WIN-05 | The pet lives on **one monitor**, chosen in settings. |
| WIN-06 | A **ground plane** at the top of the taskbar / bottom of the monitor work area; the pet is subject to gravity and lands on it. |
| WIN-07 | **Auto-hide** while a fullscreen application is in the foreground; reappear when it leaves. |
| WIN-08 | **Global hotkey** to toggle show/hide (rebindable). |
| WIN-09 | **System tray icon** with menu: show/hide, settings, quit. |
| WIN-10 | Optional **launch on Windows startup**. |
| WIN-11 | DPI-aware; all positioning and scaling in **physical pixels**. |

### 5.2 Rendering

| ID | Requirement |
|---|---|
| REN-01 | Sprites render with **nearest-neighbour, integer-only scaling** (1×–4×), independent of Windows display scaling, so pixels never blur or shimmer. Default 2×. |
| REN-02 | Animations honour **per-frame durations** from the character manifest (held frames, animating on twos/threes, smears) — there is no single fps per animation. |
| REN-03 | A **drop shadow** on the Ground plane under the Pet, which stays there while the Pet is thrown or falls. Blink, breathing and squash & stretch are drawn as Frames, not added by code. |
| REN-04 | **Emotes** above the pet that make its current motive visible (e.g. 💤 tired, ❤️ petted, ❗ noticed cursor, ❓ curious, 💢 annoyed). They are separate animated pixel art from a library in the Pack, shared by every Form, shown when the simulation signals a motive. They're optional and never catch clicks. |
| REN-05 | Render loop is frame-capped (≈12–30 fps) to respect the performance budget (§6). Cursor hit-test polling for WIN-04 runs **independently of the animation cap**, so clicks at the sprite's edge are never misrouted. |

### 5.3 Behaviour engine ("Alive")

Pure domain logic, independent of the engine, built test-first (§7.2).

| ID | Requirement |
|---|---|
| BEH-01 | **Internal drives** (e.g. energy, curiosity, boredom, playfulness, sociability). Hidden, self-regulating, never a "need" the user must tend. |
| BEH-02 | **Utility AI**: each decision tick scores available actions against current drives, personality and context, and picks one — so every action has a "why". |
| BEH-03 | **Reactive layer**: notices the cursor (look at / follow / pounce), user active vs idle, and time of day (e.g. yawns at night, perky in the morning). |
| BEH-04 | **Short-term memory** of recent interactions shapes behaviour (e.g. wary of the cursor after being thrown, clingy after petting). |
| BEH-05 | **Personality traits** fixed per character (e.g. lazy↔energetic, shy↔bold) that bias scoring. Traits come from the character pack. |
| BEH-06 | Minimum action set for the hatchling: idle (with variants), walk, sit, sleep, look-at, follow/pounce, fall, land, react-to-poke, being-petted, being-held. |
| BEH-07 | Activity-level setting scales how often/energetically it acts. |

### 5.4 Interactions

| ID | Requirement |
|---|---|
| INT-01 | **Drag & throw**: pick the pet up and fling it; momentum, bouncing off screen edges, a landing reaction. |
| INT-02 | **Petting**: stroking the pet with the cursor triggers lean-in / purr / hearts. |
| INT-03 | **Cursor play**: when playful, it chases and pounces on the cursor (laser-pointer style). |
| INT-04 | **Poke combos**: single click = poke reaction; rapid clicks escalate (giggle → annoyed → runs away). |
| INT-05 | **Right-click menu** on the pet: hide, settings, level/info, quit. |

### 5.5 Sensing and privacy

| ID | Requirement |
|---|---|
| SEN-01 | The pet may read **global cursor position** and **system idle time** (Win32 `GetLastInputInfo`) only. **Exception for WIN-07:** auto-hide may read the foreground window's geometry, class name and process ID, and whether it declares itself never fullscreen, and never its title or the application's name. |
| SEN-02 | **No keyboard hooks, no keystroke counting, no window titles, no screenshots** in the MVP. Active-app/window-title awareness arrives only with the AI phase and only as opt-in (§9, Track 6). |

### 5.6 Progression (MVP slice)

| ID | Requirement |
|---|---|
| PRG-01 | On first launch an **egg** appears on the floor. It reacts to petting (wobble) and is **hatchable immediately** — hatching is the onboarding hook. |
| PRG-02 | Hatching plays a **transformation sequence** (egg → hatchling), triggered by the user. |
| PRG-03 | The hatchling earns **XP** from interactions and active time via the XP engine (§8.2). |
| PRG-04 | **Level** is visible in the hover tooltip / right-click menu. |
| PRG-05 | Further evolutions are out of MVP scope but the form graph (§8.3) supports them from day one. |
| PRG-06 | XP is **never lost**. |

### 5.7 Settings and persistence

| ID | Requirement |
|---|---|
| SET-01 | Settings: pet scale (integer steps), monitor, activity level, launch on startup, hotkey binding, sound on/off + volume. |
| SET-02 | Pet's last position is restored on launch. |
| SAV-01 | Progress (XP, level, current form, unlocked forms, pet memory) is stored in **one local save file** with a **versioned schema and migrations**. |
| SAV-02 | **Rolling backups**; a corrupt save must never lose the pet. |
| SAV-03 | **Export / import** of the save file. |

### 5.8 Audio

| ID | Requirement |
|---|---|
| AUD-01 | Small optional sound effects (chirps, purrs, hatch, landing), **off by default**, with a volume setting. |

### 5.9 Packaging

| ID | Requirement |
|---|---|
| PKG-01 | Installable build (installer or portable package) for the owner's devices. No store listing, no auto-update. |

## 6. Non-functional requirements

### 6.1 Art bar
- **MVP:** iconic, readable character design (reference: *Skullgirls*-level iconicity) at
  **≈50 × 56–110 px** pixel art (reference: **20PAGE**). Must pass the silhouette test at 1×.
- **End vision:** hi-res pixel art around **≈380 × 610 px** where detail can pop
  (reference: **archapxl**).
- The MVP is not "done" until **one real character at the MVP bar** exists (hand-drawn, commissioned or
  pipeline-made). Placeholder art is allowed during development only.

### 6.2 Performance budget (acceptance criteria, not wishes)
| Metric | Budget |
|---|---|
| CPU while idle/animating | < 1 % of the whole machine, measured on the reference machine (§3). The spike's 0.46 % is about 15 % of one core, so a machine with fewer cores would need its own target. |
| RAM (private working set — Task Manager "Memory") | < 150 MB (raised from ~100 MB on 2026-10-30 after the spike measured a ~100 MB fixed baseline for Bevy + wgpu + DX12 driver) |
| Animation rate | ≈12–30 fps, frame-capped |
| Games / fullscreen | zero impact — pet is hidden and not rendering |
| During an Encounter | A loose guideline, not a gate: about 1.5 cores (5 % of the machine) and 250 MB, with 1,000 enemies as the normal case and 10,000 as the tested ceiling. Going over means revisiting the budget, not optimising early. The first goal is 200 enemies. Measured in [simulation-architecture.md](research/simulation-architecture.md). |

| While the Pet panel is open | A loose guideline, not a gate: about 200 MB and about 2 % CPU for the Pet and the panel together. The budgets above apply while the panel is closed, and closing it must bring the Pet back to them. Measured in [dev-panel.md](research/dev-panel.md); see [ADR-0004](adr/0004-pet-panel-as-a-separate-program.md). |

To be **revisited** if/when live Live2D/3D rendering is introduced (Track 7).

### 6.3 Robustness
- No black-window / lost-transparency failures over a full working day.
- Save file corruption is recoverable from backup (SAV-02).

## 7. Architecture

### 7.1 Stack decision (gated by a spike)

**Decision:** full Rust with **Bevy** (pin a version; upgrade deliberately — Bevy ships breaking
releases every ~3–5 months). **Fallback:** Tauri 2 (Rust shell + TypeScript/webview front end).

**Why Bevy:** best fit for the product's direction (hi-res pixel art, particles, many on-screen entities
for encounters, 3D experiments), single language, and the domain core is a plain Rust crate tested with
`cargo test`.

**Known risk:** transparent windows on Windows + NVIDIA can render black
([bevy#7544](https://github.com/bevyengine/bevy/issues/7544), open). Reported workaround: DX12 with the
DXGI Visual swapchain (`WGPU_DX12_PRESENTATION_SYSTEM=Visual`) + `CompositeAlphaMode::PreMultiplied` +
`ClearColor::NONE`. Tauri has its own open transparency issues
([tauri#16341](https://github.com/tauri-apps/tauri/issues/16341),
[tauri#15947](https://github.com/tauri-apps/tauri/issues/15947)). In both stacks per-pixel
click-through must be hand-built (poll global cursor, sample sprite alpha, toggle hit-testing).

**Spike (1–2 days, throwaway, on the reference machine — RTX 2080 Ti):** Bevy passes only if *all* hold:
1. Transparent window works with **default driver settings** (changing NVIDIA Control Panel settings = fail).
2. **Per-pixel** click-through on a real sprite, with no misrouted clicks at the sprite's edge.
3. Hidden from taskbar **and** Alt-Tab.
4. < 1 % CPU and < 100 MB RAM (since raised to < 150 MB, see §6.2) while animating at 30 fps for one hour, with no black-window failures.

If Bevy fails, run the same spike on Tauri 2.

**Spike result (2026-10-30): Bevy adopted.** Criteria 1–3 pass; criterion 4 passes on CPU (0.46 %),
fps and stability. Memory measured 101.3 MB private working set — 1.3 MB over the original budget,
which the owner then raised to < 150 MB (§6.2). The ~100 MB is a fixed cost independent of window
size; a slow creep (~1.2 MB/h) remains to be checked in a multi-hour run of the real app.
The required render configuration is recorded in
[spikes/bevy-overlay/RESULTS.md](../spikes/bevy-overlay/RESULTS.md).

Prior art worth reading: [ddbaker/neko](https://github.com/ddbaker/neko) (Bevy, Windows, mouse
passthrough), [not-elm/desktop-homunculus](https://github.com/not-elm/desktop-homunculus) (Bevy desktop
mascot), Bevy's `desk_toy` example.

### 7.2 Layering (clean architecture)

Dependencies point inward:

```
┌──────────────────────────────────────────────────────────────┐
│ Platform adapters (Win32: window flags, cursor polling,      │
│ fullscreen detection, idle time, tray, hotkey, autostart)    │
│ Engine adapter (Bevy ECS systems, rendering, audio)          │
│ Content loader (character packs from disk)                   │
│ Persistence (save file, backups, migrations)                 │
├──────────────────────────────────────────────────────────────┤
│ Domain core — framework-free Rust crate, no Bevy, no IO      │
│  • Behaviour engine (drives, utility AI, memory, traits)     │
│  • XP engine (data-driven sources, curves, caps)             │
│  • Form graph (evolutions, temporary transformations)        │
│  • Physics rules for the pet (gravity, throw, ground plane)  │
└──────────────────────────────────────────────────────────────┘
```

- The domain core is built **test-first** (Given-When-Then).
- ECS systems are thin adapters. They hold the core's types as components, call its rules, and feed in inputs
  (time, cursor, idle state). The renderer only reads their results. Swapping Bevy would keep the rules but
  rewrite those systems (see [ADR-0002](adr/0002-simulation-in-its-own-ecs-crate.md)).
- The layers are these crates (see [ADR-0001](adr/0001-os-boundary-crate.md)):
  - `pet-physics` for the physics rules and the geometry types, with no dependencies
    ([ADR-0003](adr/0003-own-physics-with-a-grab-string.md)).
  - `pet-core` for the rest of the domain core. It depends only on `pet-physics`, so it has no external dependencies.
  - `pet-content` for reading and checking Packs.
  - `pet-save` for the save file: its own types, the chain of migrations, and reading and writing.
  - `pet-sim` for the simulation's ECS systems, with no rendering.
  - `pet-platform` for the platform adapters, one folder per operating system behind an OS-neutral API.
  - `pet-protocol` for the message types of the pipe between the Pet and the Pet panel, with no workspace
    dependencies, so the panel shares nothing else with the Pet.
  - `pet-app` for everything else, including the renderer.
- The Pet panel is a separate program, `pet-panel`, built with Tauri 2, React and `pnpm`. It edits the files in the
  config folder, reads a live pipe from the Pet, and sends it a small closed set of commands
  ([ADR-0004](adr/0004-pet-panel-as-a-separate-program.md)).

### 7.3 Content packs and the character format

Characters are **content packs** loaded from a user content folder — the app ships with no hard-coded
character; the dragon is simply the first pack.

A pack contains:
- **Sprite sheets** per animation.
- **Manifest**: animations, frames, **per-frame durations**, anchor/pivot, hit-mask source, event markers
  (e.g. "footstep", "impact").
- **Form graph** (§8.3) for that character.
- **Personality traits** and per-form behaviour/trait modifiers, movement mode, (later) combat kit.
- Optional sound effects.

Requirements:
- **Users can easily add their own custom art**: documented pack format, sensible defaults, and a
  **pack validator** that reports clear errors (missing core animations, bad frame sizes, unknown fields).
- Hot-reload of packs during development is desirable (fast art iteration).
- The pack format is the **contract** between the Art & AI Pipeline track and the runtime.

## 8. Domain model

### 8.1 Behaviour engine
See §5.3. Utility AI over hidden drives, with context (cursor, active/idle, time of day), short-term
memory and fixed personality traits. Per-form trait modifiers (§8.3) feed into scoring.

### 8.2 XP engine (tunable by design)
- Every XP source (each interaction type, active time, and later focus sessions, encounters,
  achievements) is **declared as data**: base value, multiplier, optional cap / diminishing-returns
  curve, enabled/priority flag.
- XP is a **configurable matrix of rules** in `balance.toml`, in the config folder, and hot-reloaded. Each rule names
  the event kind that earns it, a base amount, context modifiers (such as ×2 for a record-high throw), a diminishing
  curve for repeats, an optional cap per hour or day, and an on/off switch. A new source is a new row, with no code.
  Item 4 starts with interactions (petting, pokes, throws, hard landings) and a one-off bonus for hatching.
- The **level curve** is data.
- A **dev balancing panel** to tweak values live and replay the formula.
- Rebalancing never requires code changes.
- **Open:** how heavily interaction XP is weighted against focus/encounter XP (see §11). The engine must
  make it trivial to promote/demote any interaction loop.

### 8.3 Form graph
- **Forms** are nodes (e.g. egg → hatchling → juvenile → adult; or human → cowgirl I → cowgirl II, …).
- **Permanent evolutions** are edges with conditions: level, consumed item, equipped relic, event.
  Linear (dragon) or branching (monster girl).
- **Temporary transformations** have a trigger, duration and cooldown (e.g. the dragon's ultra mode).
- **Unlocked forms stay switchable** — no regretted choices; collecting forms is part of the fun.
- Each form declares a **movement mode** (walk, slither, fly, hover, …), **trait modifiers**, and later a
  **combat kit**. The MVP implements **walk** only.
- **Evolutions are rituals:** when available, the pet shows a "ready to evolve" tell and waits for the
  user to trigger it — never automatic, never during a focus session. Ultra mode is combat-only.

### 8.4 Animation tiers per form
- **Core set** (mandatory — the animations the behaviour engine needs; for the hatchling see BEH-06,
  ≈11 drawn animations, since REN-03 adds only the drop shadow) and
  **extended set** (optional).
- **Transformation sequences** are their own animation type (hatch, evolve, ultra-mode in/out).
- Keeps the forms × animations art cost manageable.

## 9. Roadmap

### 9.1 MVP core (sequential)
1. **Platform spike** (§7.1) → stack locked. **Done.**
2. Window, click-through, auto-hide, tray, hotkey, autostart. **Done.**
3. Packs and the renderer, in five slices. **Done.**
   1. Pack format, `pet-content` and the validator.
   2. `pet-sim` with its phases and the Animation clock, and a renderer with integer scaling and Anchor placement.
   3. The drop shadow.
   4. Emotes.
   5. Hot reload.
4. **The living Pet, in nine slices** ([item-4-architecture.md](proposals/item-4-architecture.md)). The old items 4
   (domain core), 5 (interactions) and 6 (settings and save) are re-cut, so the owner can interact with the Pet and
   keep its progress from the start:
   1. Ground + throw: `pet-physics`, the Ground plane, work-area changes, Obstacles, Materials and the grab string.
   2. Save: `pet-save`, one versioned JSON file with a checksum, migrations and rolling backups.
   3. Petting + hatching: strokes, the Form graph, and the egg hatching when the owner pets it enough.
   4. Reactions: Actions with timing, effects and their own Body, played in answer to Gestures.
   5. XP + Level: the XP matrix, the level curve, the right-click menu, and one hot reload system for every file.
   6. Pet panel: the separate Tauri program and its live pipe, with Progress and Balance tabs.
   7. The Mind: Pack drives and traits, a tree of utility choices, Emotes as reasons, and the Mind tab.
   8. Senses + memory: cursor, idle time and time of day; two-speed memory; learned Links, Experiments and the
      Marker.
   9. Settings + the rest: the Settings tab, export/import, optional SFX, and an installable build (PKG-01) that
      includes the panel program and WebView2. Settings include the Return delay (`RETURN_DELAY` in `pet-core`).
5. First real character pack: egg + hatch sequence + hatchling at the MVP art bar.

Development on item 4 runs against placeholder art and Brim; 5 runs in parallel and gates "MVP done". Each slice of
item 4 ends with something the owner can do to the Pet by hand, as well as its automated tests.

### 9.2 Parallel tracks (post-MVP, pulled in whenever desired)

Each track has its own backlog; increments are added as they feel right.

| # | Track | Highlights |
|---|---|---|
| 1 | **Behaviour & Personality** | Richer drives, longer-term memory, more context reactions, more idle variety. The planner that chains learned Links into Routines, and the visual Mind editor in the Pet panel. |
| 2 | **Interactions** | Toys (ball/yarn with physics), feeding (treat pouch), naming, tricks (after leveling). |
| 3 | **Encounters & RPG** | See §10.2 — portals, co-op combat, loot, relics, ultra mode, expeditions, dungeons. Exploratory: prototype before specifying. |
| 4 | **Productivity** | Focus/Pomodoro timer, break & stretch reminders, hydration reminders — delivered by the pet, not system toasts. Calendar-aware do-not-disturb later (needs OAuth). |
| 5 | **Art & AI Pipeline** | See §10.3. |
| 6 | **AI Companion** | See §10.4. |
| 7 | **Platform** | Multi-monitor roaming, window-aware movement (sit on title bars, fall when windows move), screen-share detection, live 3D / Live2D rendering, macOS/Linux. |

## 10. End vision

### 10.1 What it becomes
A companion that hatches on your desktop, grows up alongside your work, transforms in ways you can see,
fights at your side, and eventually talks with you — while never getting in your way.

### 10.2 Encounters & RPG (hypotheses — to be prototyped, not yet specified)
- **Core-loop hypothesis:** focus session → a portal opens → you and the pet fight together → XP and
  loot → level up / evolve → next focus session. No encounter ever starts during a focus session;
  encounters also available on demand. Skipping or failing costs nothing.
- **Encounter shape:** orcs emerge from a portal and walk the same ground plane; the pet fights via its
  utility AI; you fight alongside with clicks and 1–2 cooldown abilities; 60–90 s; difficulty scales with
  level; mouse-only so it never collides with work shortcuts; click-through still applies outside
  combat entities. Start with one enemy type and one portal style.
- **Synergy principle:** *every interaction verb also means something in combat.* First hypotheses:
  - **Throw-as-attack** — fling the pet into enemies; good aim crits.
  - **Sync strikes** — click an enemy inside the pet's attack window for a combo multiplier.
  - **Bond meter** — out-of-combat play (petting etc.) charges stronger joint moves in combat.
  - Further candidates: pet calls for help, combo abilities (you freeze / it shatters), petting as a
    mid-fight rally/heal, drag-to-command targeting.
- **Ultra mode:** temporary transformation (e.g. the dragon's power-up), combat only.
- **Loot economy:** consumables (food) and relics (equippable); both can trigger evolutions; plus
  cosmetic loot.
- **Expeditions (first):** during a focus session the pet sends a **shadow/familiar** on an idle dungeon
  run (or it plays out in a small corner vignette) — the pet itself stays with you. Loot is delivered
  after the session.
- **Active dungeons (later):** playable runs between sessions, an extension of portal encounters.

### 10.3 Art & AI Pipeline
> A semi- to fully-automatic pipeline that produces iconic, alive-feeling animated characters (pixel art
> or 3D) meeting the art bar, outputting assets that conform to the runtime's character format.
> Evaluated per character on: silhouette and readability at desktop scale, consistency across all
> animations, loop quality, and **human effort per new animation / per new form**.

- Approaches are open: hand-drawn, AI-assisted pixel art, or **3D models baked offline into sprite
  sheets** through a pixelization/toon shader (references: *Dead Cells*, t3ssel8r).
- **3D source assets are baked offline** — the runtime stays sprite-only. Live 3D is a Platform-track
  experiment.
- **Stylized animation is a hard requirement** regardless of source: per-frame timing, held frames,
  animating on twos/threes, smear frames, squash & stretch, exaggerated anticipation/follow-through,
  and a **paint-over / hand-correction pass** on any generated or baked frame. "Boring" joint-interpolated
  3D motion does not meet the bar.
- The MVP does **not** depend on this track; it swaps in when output clears the bar.

### 10.4 AI Companion
- **Interface:** text first, voice later. Whether text appears as speech bubbles or another visual
  language is open (§11).
- **Model:** cloud LLM (Claude) with an optional local-model provider behind the same interface.
- **Context access — tiered and opt-in:**
  1. Default: only what you type.
  2. Opt-in: active app name + window title.
  3. Explicit, per-request only: a screenshot.
- Privacy gets a dedicated design section when this track starts.

### 10.5 Characters
1. **Dragon (first pack):** egg → hatchling → … → adult; ultra mode as a temporary transformation.
2. **Monster-girl character (later pack):** a human girl who transforms by consuming items or equipping
   relics found in dungeons / won from enemies — into cow girl, lamia, dragon girl, centaur, mermaid,
   harpy, orcess, etc. — each with multiple evolutions. Forms change movement mode (slither, fly, swim…),
   traits and combat kit. Unlocked forms stay switchable.

### 10.6 A Mind that learns (north star, 2026-11-06)
The Mind should come to understand its surroundings and the situations it is in, learn fast from few tries, and
remember for a long time. What it can do grows by combining general Actions in ways nobody wrote down, learned from
play with the owner and from its own Experiments (§2.1, [ADR-0005](adr/0005-a-learning-mind-with-reasons.md)). The
owner can also teach it, by rewarding what it does and marking the moment that was right.

Two illustrations from the owner, examples only and not requirements:
- After a little practice of being thrown and then smashing, the Pet comes to the owner's cursor in a fight. Once
  thrown over a group of enemies, it smashes when it is above their middle.
- When the owner throws a ball, the Pet learns to shoot at it, because that gets rewarded.

The MVP builds the foundations: Features, learned Links, Experiments, the Marker and decision records. It also ships
one real loop in companion mode: the Pet notices which of its Actions the owner rewards, and in which situation, and
does them more then, and says why. Track 1 adds the planner that chains Links into Routines. How the Pet *tells* the
owner what it wants is a separate system: Emotes now, natural language with the AI track (§10.4).
Research: [pet-memory.md](research/pet-memory.md), [mind-architecture.md](research/mind-architecture.md),
[teachable-mind.md](research/teachable-mind.md), [neural-mind.md](research/neural-mind.md).

## 11. Open questions

| # | Question | Notes |
|---|---|---|
| OQ-1 | Weighting of interaction XP vs focus/encounter XP | Engine is data-driven so this can be explored by tuning. Watch for "playing instead of working". |
| OQ-2 | How the pet communicates beyond emotes | Text speech bubbles vs another visual language; depends on the pet's intelligence level and aesthetics. |
| OQ-3 | Encounters & RPG design | Core loop, synergy mechanics and economy are hypotheses — validate via throwaway prototypes. |
| OQ-4 | Art route for the first character | Hand-drawn, commissioned or pipeline — whichever reaches the MVP bar first. |
| OQ-5 | Final product name | Placeholder "Desktop Pet". |
| OQ-6 | Character concept for the dragon | Fill in the brief (Appendix A). |

## 12. Decision log

| Decision | Rationale |
|---|---|
| Ambient companion MVP → productivity → AI (+ RPG tracks) | Making a pet feel alive on the desktop is the hard, foundational part. |
| Personal-first, distributable-later | Avoids premature store/update work without closing doors. |
| Windows 11 only for MVP | Always-on-top, click-through and transparency all differ per OS. |
| 2D sprites at runtime; 3D only baked offline | Keeps the performance budget and a single runtime format. |
| Ground plane, not window-aware, for MVP | Window tracking is substantial platform work; saved for Track 7. |
| No needs / no guilt; XP never lost | Clashes with "keeps me company while I work". |
| Utility AI + drives + memory + traits | Random state selection felt unintelligent; reasoning must be visible. |
| Cursor + idle time only | No keyboard hooks (privacy, antivirus heuristics, keylogger optics). |
| Core + parallel tracks roadmap | Owner pulls increments as they feel right. |
| Integer-only nearest-neighbour scaling | Pixel art must stay crisp on a 4K display. |
| Bevy, gated by spike; Tauri 2 fallback | Best fit for vision; transparency risk is explicitly tested on worst-case hardware. |
| Bevy adopted; RAM budget raised to < 150 MB (2026-10-30) | Spike passed everything else; Bevy + wgpu + DX12 driver has a ~100 MB fixed baseline, so 150 MB leaves headroom for real art and systems. |
| Characters as user-addable content packs | Art independence from the binary; custom art; easy curation for any distribution. |
| Form graph with switchable forms; evolutions as rituals | Growth/transformation is the design signature; no regret, never in the way. |
| Egg hatchable immediately | Hatching is the onboarding hook. |
| Local save only, no sync | Single-device simplicity; export/import covers moving devices. |
| SFX off by default | Respects calls/music on a work machine. |
| Ground plane moved from item 2 to item 4 (2026-11-01) | It's pet physics, which belongs in the domain core. Item 2 uses a stationary placeholder. |
| CPU budget gated on the reference machine (2026-11-01) | Only that machine has been measured; the figure is a share of its 32 threads. |
| Auto-hide reads foreground window geometry (2026-11-01) | WIN-07 can't work without it. Titles and application names stay off limits. |
| Auto-hide looks at the Home monitor only (2026-11-01) | Multiple monitors can't be tested on the only machine available, so there's no code for them. |
| OS boundary is a crate with one folder per OS (2026-11-01) | Keeps macOS/Linux additive. See [ADR-0001](adr/0001-os-boundary-crate.md). |
| Hotkey through `RegisterHotKey` on a message window, not the `global-hotkey` crate (2026-11-01) | That crate polls `GetAsyncKeyState` after each press. A window also keeps a press that arrives during the tray menu. |
| Hidden stops everything: window, camera, updates and the hit-test poll (2026-11-01) | "Zero impact" (§6.2) should mean it. The world freezes, so the Pet reappears where it was. |
| One Pet per Windows session, using a `Local\DesktopPet` mutex (2026-11-01) | A second launch shows the first Pet and exits. |
| Autostart is one `Run` value, and `StartupApproved` is never touched (2026-11-01) | If the owner turns the app off in Windows Settings, that wins. |
| Relaunch after a render failure, at most 3 times in 10 minutes (2026-11-01) | Recovers from a GPU reset, without looping for ever on a machine that can't render. Keeps a Manual hide. |
| Warnings and errors go to `%LOCALAPPDATA%\DesktopPet\desktop-pet.log` (2026-11-01) | The release build has no console, so this is the only record of a failure. |
| REN-03 cut to a drop shadow; Emotes are separate art (2026-11-01) | Blink, breathing and squash & stretch need character-specific data or can't stay on the pixel grid, and hand-drawn Frames do them better. |
| Packs are data only, with no scripting (2026-11-02) | A new kind of effect needs Rust. Scripting can come later without changing the design. |
| One Pet on screen; Progress kept per Character (2026-11-02) | Switching Characters must never lose progress (PRG-06). |
| Encounters get their own budget, and companion mode stays light (2026-11-02) | The owner is focused on a fight, but works alongside the Pet the rest of the time. |
| The simulation runs on Bevy's ECS in a `pet-sim` crate, and its rules stay in `pet-core` (2026-11-03) | Measured: speed doesn't separate the options, so coupling did. See [ADR-0002](adr/0002-simulation-in-its-own-ecs-crate.md). |
| No physics engine; rectangles for the world and combat, pixels for clicks; 30 ticks a second (2026-11-03) | Nothing rotates, stacks or hangs on joints. A swept test stops fast Projectiles passing through. Amended on 2026-11-06 by [ADR-0003](adr/0003-own-physics-with-a-grab-string.md): own physics in `pet-physics`, a grab string. |
| No built-in Actions, only a short list of Situations (2026-11-03) | The Mind's choices are Pack data. Only what the simulation causes itself (held, falling, landing) needs a fixed name. |
| During an Encounter the Overlay catches every click on the Home monitor (2026-11-03) | A stray click mustn't reach an app underneath. The fight stays on the desktop, and Escape forfeits. |
| A Pack loads completely or not at all; until Settings exist the first by folder name is used; the tray icon is the app's own (2026-11-03) | A half-loaded Pet is harder to understand than a clear error list. The tray must work with no Pack, so you can still quit. |
| Packs hot-reload in release builds too (2026-11-03) | A file watcher costs nothing while idle and helps anyone making their own art. A Pack that fails validation leaves the old one in place. |
| Pixel art draws as Mesh2d, never Bevy's Sprite (2026-11-04) | On DX12 the sprite shader reads every draw after the first from the wrong slot (gfx-rs/wgpu#4564). Mesh2d reads it correctly and gives the same pixels. |
| Half-transparent pixels blend in sRGB, as an image editor shows them (2026-11-04) | Pixel art should look as it was drawn. Opaque pixels are the same either way. |
| Animation clocks count whole ticks, and every Frame lasts at least one tick, 34 ms (2026-11-04) | A Frame then lasts exactly as long as the Pack says and is never skipped. `pet-pack validate` rejects shorter Frames. |
| Emotes are asked for with one ordered message, `EmoteRequest` (2026-11-04) | A hide and a show in the same tick then mean what they were sent to mean. The target keeps an Actor on it, so enemies can show Emotes in item 4. |
| On a Pack reload, a Pet still in its starting Form follows a changed `starting_form`; any other Form is kept by name (2026-11-06) | Otherwise an author who changes `starting_form` sees nothing. Only the Form the Pet is in now counts, so two edits can still move a grown Pet back; item 4's Progress must close that. Slice 3 closes it, with a saved flag for having left the starting Form. |
| Items 4–6 re-cut into one item, "The living Pet", in nine slices; the real character becomes item 5 (2026-11-06) | Interacting with the Pet is how the owner checks the Mind, memory and senses, and saving is needed as soon as the Pet hatches. The slices crossed the old item boundaries, so three numbers would mislead. |
| Obstacles: solid rectangles on all four sides, listed by hand in `obstacles.toml` and drawn as faint outlines (2026-11-06) | One general kind of thing the Pet collides with and stands on, so the physics is general and the swept test can be tried by hand. The Ground plane and screen edges are the first ones. Window rectangles join the list in Track 7. |
| Hand-edited files live in `%LOCALAPPDATA%\DesktopPet\config\` (2026-11-06) | Windows watches folders, not files, and the log next to them is written all the time. Its own folder keeps the watcher asleep. |
| The save is one JSON file with a checksum, not SQLite (2026-11-06) | At 5,000 records JSON loads in 3.2 ms and saves in 6 ms, against SQLite's 2.6 ms and 1.1 ms per change; nothing measured comes near the budget. JSON adds 187 KB against 1.8 MB and the app's first C compile, and stays readable. Without a check, all 20 bit flips loaded wrong data, so the checksum is required. Revisit at about 10 MB. See [persistence.md](research/persistence.md). |
| Saves are written at most 2 s after a change, and at once on quit or hide; backups at each launch and daily (2026-11-06) | A power cut loses at most 2 s, which bends PRG-06 by that much. The last 3 launch copies and 7 daily ones are kept. A file that fails to load is set aside and never overwritten. |
| Persistence gets its own crate, `pet-save` (2026-11-06) | Its file types, migrations and IO stay out of `pet-core`, as `pet-content` does for Packs. |
| Physics is our own code in a `pet-physics` crate with upright boxes (2026-11-06) | It was the fastest and smallest option measured, and it fits ADR-0002. It amends the 2026-11-03 decision for the grab string and choices while held or falling. See [ADR-0003](adr/0003-own-physics-with-a-grab-string.md). |
| Every surface and Body has a Material, and an Action's override beats the combine rule (2026-11-06) | Sliding down a wall, bouncing off it and a hero landing that never bounces all need it. |
| Grabbing is a short springy string from the cursor; the Pet never rotates (2026-11-06) | Flicking and swinging build momentum without spinning the art or the Body, so the Hit mask, the shadow and the Emotes are unaffected. |
| The Pet panel is a separate Tauri 2 + React program that edits files; its pipe carries live data out and a closed set of commands in (2026-11-06) | It's the only option that costs nothing when closed, and WebView2 gives the most creative freedom. Commands cover what the Pet owns, such as Evolve and import, which files can't change safely. The message types live in `pet-protocol`. See [ADR-0004](adr/0004-pet-panel-as-a-separate-program.md). |
| PKG-01 in slice 9 (2026-11-06) | It was in no item before; packaging now has to include the panel program and WebView2. |
| The Mind learns, "why" is a hard rule, and Experiments are seeded and labelled; pillar 1 reworded (2026-11-06) | Capabilities should emerge from general Actions, which needs learning and some exploration, without losing visible reasons. See [ADR-0005](adr/0005-a-learning-mind-with-reasons.md). |
| A middle click on the Pet is the Marker, and its worth is learned from the petting that follows (2026-11-06) | Petting is too slow to mark a split-second choice and interrupts what the Pet is doing. A plain click is a poke, and a hotkey would break mouse-only play. |
| Memories consolidate during any Action tagged `rest`, plus a background timer whose rate is a balance number (2026-11-06) | Sleep stays free for the productivity features, and a Pet that rarely rests still consolidates. |
| The Pet's lasting opinion of the owner never goes below 0 (2026-11-06) | It can be lastingly wary of the cursor and sulk for a while, but never resentful of the owner, which keeps "no guilt" and "better together". |
| Rhythm learning and a local decision log are on by default, visible in Settings, and can be reset or turned off (2026-11-06) | Both are profiles of the owner, kept on the machine only. The log, about 50–300 KB a day for 30 days, is how the owner checks what the Mind did and what later tuning tools learn from. |
| One Mind format for the Pet and every enemy (2026-11-06) | One editor and one validator. Simple enemies use only priority nodes. |
| The owner's Mind edits are a per-Character override over the Pack (2026-11-06) | A Pack update can't wipe them, and a bad edit can't fail the Pack's validation. What the Pet learned is a third, separate layer in Progress. |
| One hot reload system serves Packs, `balance.toml`, `obstacles.toml` and `settings.toml` (2026-11-06) | It's written once, and the Pack is the first source moved onto it. The save file is excluded because the app writes it. |
| `format_version` stays 1 through the MVP (2026-11-06) | Only the owner's Packs exist. Bumps and migrations for Packs start when Packs are shared. |
| While the owner holds the Pet, the Overlay catches clicks wherever the cursor is, until the hold ends or Windows takes the mouse away (2026-11-07) | On the grab string the Pet trails behind the cursor, so its pixels slip out from under it. The hold mustn't break, nor a click reach the app underneath, just because they did. |
| Slice 5 chooses the landing speed that counts as an impact (2026-11-07) | XP is the first thing to use impacts, so the number is tuned there. Until then each landing only reports its speed, and nothing is called an impact. |

## 13. References

**Art bar**
- 20PAGE (pixel artist) — MVP target scale and style.
- archapxl (pixel artist) — end-vision hi-res pixel art target.
- *Skullgirls* — iconic character design.

**Technique references**
- Small frame-by-frame pixel art: *Celeste*, *Stardew Valley*, *Rusty's Retirement*, Shimeji, oneko.
- Detailed pixel art: *Eastward*, *Owlboy*, *Blasphemous*, *Sea of Stars*.
- 3D → pixel art: *Dead Cells*, t3ssel8r, *A Short Hike*.
- Hand-drawn cel: *Hollow Knight*, *Cuphead*, *Spiritfarer*, *Skullgirls*.
- Skeletal 2D: *Cult of the Lamb*, *Darkest Dungeon*, *Slay the Spire*, Live2D VTuber models.
- Desktop companions: *Desktop Mate*, *Spirit City: Lofi Sessions*.
- Pipeline tooling to evaluate: PixelLab, Retro Diffusion, Meshy, Tripo, Mixamo, DeepMotion,
  Rokoko Vision, Move.ai.

**Stack research (checked 2026-10-29; Bevy 0.20.1, Tauri 2.12.5)**
- [bevy#7544](https://github.com/bevyengine/bevy/issues/7544), [bevy#25366](https://github.com/bevyengine/bevy/issues/25366) — transparent window on Windows/NVIDIA.
- [tauri#16341](https://github.com/tauri-apps/tauri/issues/16341), [tauri#15947](https://github.com/tauri-apps/tauri/issues/15947), [tauri#2090](https://github.com/tauri-apps/tauri/issues/2090) — transparency and click-through.
- [Bevy `Window`](https://docs.rs/bevy/0.20.1/bevy/window/struct.Window.html), [`CursorOptions`](https://docs.rs/bevy/0.20.1/bevy/window/struct.CursorOptions.html), [`WinitSettings`](https://docs.rs/bevy/0.20.1/bevy/winit/struct.WinitSettings.html).

---

## Appendix A — Character concept brief (template)

Fill one in per character pack, whether hand-made or pipeline-made.

| Field | Content |
|---|---|
| **Name / species** | |
| **One-line pitch** | |
| **Silhouette test** | Recognisable as a solid black shape at 1× scale? Key silhouette features: |
| **Palette** | Limit (e.g. 16–24 colours); key colours: |
| **Personality traits (2–3)** | Each mapped to a behaviour-engine trait (e.g. "curious" → high curiosity drive gain). It must *look* how it *acts*. |
| **Signature pose** | |
| **Signature idle** | |
| **Forms** | Form graph sketch (MVP: egg → hatchling). |
| **Transformation sequences** | e.g. hatch, evolve, ultra-mode in/out. |
| **Core animation set** | idle (+variants), walk, sit, sleep, look-at, follow/pounce, fall, land, poke reaction, petted, held — plus egg idle/wobble and hatch sequence for the MVP. |
| **Extended animations** | |
| **Movement mode per form** | |
| **Sound palette** | |
