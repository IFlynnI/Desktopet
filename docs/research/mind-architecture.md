# Mind architecture: which shape, and how a user edits it

**Researched:** 2026-11-06, for roadmap item 4 (BEH-02, §8.1) and the Mind kinds in GLOSSARY.md.
**Purpose:** compare eight ways to build a Mind on two counts: how well each fits this product, and how well a
non-programmer can author it in a visual editor. Then recommend a runtime model and a data format, so the MVP's Mind
can be edited in a UI later without a rewrite.

**What the owner asked for.** Users should be able to configure the Pet's Mind themselves through a UI, "something like
behaviour trees but more advanced". Pack authors add custom Actions, which must plug into the Mind. Enemies in the RPG
mode use simple Reflex Minds; bosses use utility plus phases.

| Mark | Meaning |
|---|---|
| **Read** | I read the primary text for this doc. |
| **Summary** | I found the primary source but read only its listing, a search snippet or a write-up. Take the lesson, not the details. |
| **Inference** | My reasoning or estimate. Not measured. |

Context7 wasn't available in this session. Crate facts come from the crates.io API and GitHub, checked 2026-11-06.

## Bottom line

- **"More advanced than behaviour trees" means three different things in industry**: utility scoring inside a tree,
  Unreal's StateTree, or a planner (GOAP, HTN). This product needs the first two together: **a StateTree-like
  structure whose choices are made by Infinite Axis utility scoring.** Planners are the wrong kind of "advanced" here.
- **The biggest single discriminator is how a Pack's Action joins the Mind.** In a pure behaviour tree, someone has to
  edit the tree for every new Action. A utility choice whose options are "every Action with tag `play`" lets a new Action
  join as data, with no tree edit. That is close to how The Sims' objects advertise, and to Halo 2 inserting per-character behaviours.
- **The MVP builds one node kind**, the utility choice, as a tree of depth one. That is the drafted `ActionDef` list
  plus a `tags` field. The data file is a node list from day one, so phases, sequences and Reflex priority lists are added node
  kinds later, not a new format.
- **Cost doesn't decide it, except for planners at 1,000 enemies.** Everything else fits in microseconds. **Inference.**
- **Own the code in `pet-core`.** No Mind crate targets Bevy 0.20 *and* loads its tree from data *and* has utility
  selection. The editor can use `egui-snarl` with `bevy_egui`, which both work with Bevy 0.20 today.

## The recommended shape, in one picture

```text
 Situations: held, falling, landing      fixed in Rust, always above the Mind; a user can't move or remove them
        │ none active
        ▼
 ┌─ Mind (Pack data, editable) ──────────────────────────────────────────────────────────┐
 │  states  "Pet modes" or "boss phases"      one active child; transitions on conditions   │
 │    ├─ day ─► utility  options: tags [idle, play, rest]   highest score wins, fixed ties  │
 │    │            ├─ Action walk      considerations ─► score 0.41                          │
 │    │            ├─ Action chase     considerations ─► score 0.72  ◄── wins               │
 │    │            └─ sequence "nap"   walk_to_edge → sit → sleep                           │
 │    └─ night ─► utility  options: tags [rest, idle]                                       │
 └─────────────────────────────────────────────────────────────────────────────────────────┘
 A Reflex enemy is a separate Mind in the same format: priority, first child whose conditions pass.
        │ every decision returns: path, winner, top factors and their causes from memory
        ▼
 "why": day ▸ chase 0.72 ← playful 0.8, opinion(cursor) +0.4 (petted ×2, 3 min ago)  ──► Emote, tooltip, dev panel
```

## 1. The eight candidates, with a web analogy each

| Architecture | In plain words | Web analogy |
|---|---|---|
| **Utility AI, Infinite Axis** (Mark; Lewis, GW2) | Score every option by multiplying curves over inputs; take the best. | A search engine ranking results, with each result's ranking signals shown. |
| **Behaviour tree** (Halo 2, Unreal BT) | A priority list of branches; the first whose condition passes runs. | Express middleware or a router: the first matching route handles the request. |
| **Hybrid utility BT** (Merrill) | A behaviour tree where some selectors rank their children by score. | A router where one route is a search page that ranks what it shows. |
| **StateTree** (Unreal 5) | Nested states with enter conditions and transitions; a parent picks its child by order, or by utility (documented for UE 5.5; first version not checked). | Nested routes with guards (XState meets React Router). |
| **HTN planning** (Transformers: Fall of Cybertron, Horizon Zero Dawn) | A goal splits into subtasks by the first method whose conditions pass, down to primitive tasks: a plan. | `make`: a target expands into recipes for its dependencies. |
| **GOAP** (F.E.A.R.) | A* searches for a chain of actions whose effects reach a goal. | A SQL query planner picking the cheapest plan to reach the result. |
| **Hierarchical state machine** | States inside states; each state lists its exits. | A statechart, or nested UI routes with explicit links. |
| **Creatures** (Grand et al. 1996) | A neural net picks the strongest of 16 decision cells; learns from strokes and slaps. | A trained ML model in production: it works, but nobody can edit a rule. |

## 2. What each source teaches

| Source | Lesson for this product | Mark |
|---|---|---|
| Dawe, Gargolinski, Dicken, Humphreys & Mark, [Behavior Selection Algorithms](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter04_Behavior_Selection_Algorithms.pdf), Game AI Pro ch. 4 | BTs: behaviours "unaware of each other", easy to extend, and "a child behavior could be run based on a utility system-type selector". Utility: flexible but "challenging to tune", "more art than science". GOAP and HTN take "authorial and directorial control" away from designers. HTN lets designers "more easily read through" domains. | Read |
| Merrill, [Building Utility Decisions into Your Existing Behavior Tree](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter10_Building_Utility_Decisions_into_Your_Existing_Behavior_Tree.pdf), Game AI Pro ch. 10 | In a BT "priority is static", so authors duplicate branches. The fix is a **utility selector** node; children report utility, decorators can reshape it (square, cube). His team left a planner because it was "too organic and mysterious for the designers' comfort". | Read |
| Isla, [Handling Complexity in the Halo 2 AI](https://www.gamedeveloper.com/programming/gdc-2005-proceeding-handling-complexity-in-the-i-halo-2-i-ai), GDC 2005 | A behaviour DAG of ~50 behaviours with prioritised lists. Relevancy is a yes/no test, because tuning floats is near impossible past ~20 options. Per-character behaviours are inserted in a preprocessing step. Designers pick "styles" from a small approved library, since styles are powerful and dangerous. | Summary |
| Humphreys, [Exploring HTN Planners through Example](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter12_Exploring_HTN_Planners_through_Example.pdf), Game AI Pro ch. 12 | The HTN in Transformers: Fall of Cybertron needed no cost sorting, so it was "considerably faster" than the GOAP in War for Cybertron. Replanning on every world change brings back bugs, and method order matters. | Read |
| Orkin, [Three States and a Plan](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf), GDC 2006 | Three FSM states plus an A* planner. Designers assign a **Goal Set** and an **Action Set** in a database editor; soldier, assassin and rat get different behaviour from the same goals. Each action is a C++ class. | Read |
| Mark & Lewis, [Building a Better Centaur](https://www.gdcvault.com/play/1021848/), GDC 2015; Mark & Dill, [Improving AI Decision Modeling Through Utility Theory](https://www.gdcvault.com/play/1012410), GDC 2010; [gameai.com/iaus](https://gameai.com/iaus.php) | The Infinite Axis Utility System is "entirely data-driven", authored in a database-style tool, shared across characters. Field names and the scoring rules are already in [simulation-architecture.md §5](simulation-architecture.md#5-utility-ai-prior-art), including Lewis's point that most curves came from presets. | Summary |
| Epic, [Behavior Tree overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/behavior-tree-in-unreal-engine---overview) | Unreal BTs are event-driven: conditions belong in decorators, which observe values and abort branches, instead of being polled. Services run while their subtree is active. Simple Parallel: "while doing A, do B". | Summary |
| Epic, [StateTree selectors](https://dev.epicgames.com/documentation/en-us/unreal-engine/state-tree-selectors-overview) and [selection enum, 5.5](https://dev.epicgames.com/documentation/unreal-engine/API/Plugins/StateTreeModule/EStateTreeStateSelectionBehavior?application_version=5.5) | "Try Select Children With Highest Utility": among children whose enter conditions pass, the top score wins and ties go to list order. Considerations output 0–1, combine with AND/OR, then multiply by a weight. A weighted-random mode also exists. | Summary |
| Guerrilla, [The AI of Horizon Zero Dawn](https://www.guerrilla-games.com/read/the-ai-of-horizon-zero-dawn), [HTN Planning in Decima](https://www.guerrilla-games.com/read/htn-planning-in-decima) | Machines request plans from an HTN planner for goals such as attack or flee; a herd agent coordinates groups. The planner backtracks "similar to Prolog", and its domains compile to generated C++. | Summary |
| Grand, Cliff & Malhotra, [Creatures](https://www.sussex.ac.uk/informatics/cogslib/reports/csrp/csrp434.pdf), 1996 | About 1,000 neurons in 9 lobes and 5,000 synapses; 16 decision cells; the strongest wins and runs an action *script*. Neuron rules are interpreted opcodes. Up to ten creatures on a 486 before slowdown. | Read |
| Editors: [Behavior Designer Pro Utility Selector](https://opsive.com/support/documentation/behavior-designer-pro/concepts/tasks/composite/utility-selector/), [NodeCanvas BT reference](https://nodecanvas.paradoxnotion.com/documentation/?section=bt-nodes-reference) and [FSM reference](https://nodecanvas.paradoxnotion.com/documentation/?section=fsm-node-reference), [Unity Behavior](https://docs.unity3d.com/Packages/com.unity.behavior@1.0/manual/node-types.html) | Two commercial Unity BT tools added utility selectors with curve-based evaluators. NodeCanvas lets an FSM state hold a whole sub-BT. Unity Behavior writes nodes as plain-language sentences, but custom nodes are C#. | Summary |

## 3. Product fit

Costs are **Inference**, anchored to two measured figures: movement for 200 enemies costs about 5 µs a tick
([simulation-architecture.md](simulation-architecture.md#bottom-line)), and companion mode has about 5.5 ms of headroom
per 33 ms tick ([pet-memory.md](pet-memory.md#bottom-line)). I assume one consideration costs 20–50 ns (one input read,
one curve), and that a Mind decides on events and every ~0.5 s (the drafted `next_decision`), not every tick.

| | Visible "why" | Pack Action plugs in by | Pure data, no scripts | One Pet, per decision | 1,000 enemies, per tick | Fit with pet-memory.md's design B |
|---|---|---|---|---|---|---|
| **Utility (IAUS)** | **Yes, the best**: scores and top factors | Being added: every Action is an option | Yes | 20 Actions × 4 considerations: ~2–4 µs | Full utility, 10 options × 3 targets: ~2.7 ms if all decide every tick, ~0.3 ms staggered over 10 ticks | **Native**: memory reads are float inputs with causes |
| **Behaviour tree** | Yes, as a path of passed yes/no tests; no "how much" | Editing the tree, or a Halo-style insertion step | Yes, with a closed node list | ~1 µs | ~0.2–0.4 ms ticked from the root; much less event-driven | Thresholds only: `opinion(cursor) < −0.3` loses gradation |
| **Hybrid utility BT** | Yes: path plus scores | A tagged utility selector | Yes | ~2–4 µs | As BT for the cheap parts | Native inside utility selectors |
| **StateTree + utility** | Yes: active state path plus scores | A tagged utility selector | Yes | ~2–4 µs; transitions checked on events | ~30–100 µs as Reflex priority lists | Native, and conditions use thresholds |
| **HTN** | Partly: "goal → method → tasks" explains how, not why that goal | Adding methods, whose order then matters | Mostly; conditions and effects become a small language | ~10–100 µs per plan | 1,000 replanning at once, e.g. on a boss's death: ~10–100 ms spike; must cap plans per tick | Memory becomes world-state facts |
| **GOAP** | Weakly: "why this plan" is an A* cost | **Easily**: an Action with preconditions and effects is found by the planner | Mostly, but actions in F.E.A.R. were C++ classes | ~50 µs–1 ms per plan | Same spike, worse: ~50 ms–1 s if all replan at once | Memory becomes world-state facts, like F.E.A.R.'s Facts |
| **HFSM** | Yes: current state plus the transition that fired | **Poorly**: a new state needs exits to and from others | Yes | <1 µs | ~30 µs: only the current state's exits | Thresholds only |
| **Creatures** | **No**: a neuron fired | Not really: a new decision cell needs rewiring and retraining ([neural-mind.md](neural-mind.md) reaches the same verdict) | No: SVRules are interpreted opcodes, and actions are scripts | ~5–20 µs | ~5–20 ms | It *is* the memory, and would replace design B |

**Per tick for one Pet:** a utility decision of ~2–4 µs about once every 15 ticks averages under 0.3 µs a tick; the
worst tick, with one decision in it, is ~4 µs, under 0.1 % of the headroom. **Inference.**

**What rules out candidates here, beyond the table:**

- **Nothing random (pillar 1, pet-memory rule 2).** StateTree's weighted-random mode and Dill's weighted pick are out.
  Highest score wins and ties break in list order, as StateTree's utility mode already does.
- **No scripting (decision log, 2026-11-02).** Isla's impulses that run code, Unity Behavior's C# nodes and Creatures'
  SVRules are all scripting. The node vocabulary must be a closed list in Rust.
- **Situations sit above the Mind (decision log, 2026-11-03).** Held, falling and landing work like Zoo Tycoon 2's
  dying rank or Unreal's observer aborts: always first. The editor must not let a user put anything above them.
- **Forms add Actions (GLOSSARY.md).** Switching Form changes the option set at once. Tag-based options handle that
  for free; a hand-wired tree would need a copy per Form.

## 4. Authoring by a non-programmer

| | What the author edits | Real editor to copy | Hard part for a non-programmer |
|---|---|---|---|
| **Utility (IAUS)** | Per Action: weight, considerations, each an input, bookends and a curve | Curvature; Dave Mark's database tool | Tuning many interacting floats. Isla: near impossible past ~20 options. Lewis: use presets |
| **Behaviour tree** | A tree of selectors, sequences, conditions | Unreal BT: canvas, decorators and services as boxes on nodes, live path highlight; Behavior Designer; Unity Behavior's sentence nodes | Static priority: duplicating branches for context (Merrill). Easy to read, tedious to grow |
| **Hybrid utility BT** | Tree, plus curves on some selectors | Behavior Designer Pro's Utility Selector; NodeCanvas Priority Selector with considerations and curves | Two concepts to learn, but each piece is small |
| **StateTree + utility** | An outline of states; tasks, enter conditions, transitions; considerations on children | Unreal StateTree: an indented outline, not a free canvas, with details panels | Transitions can sprawl if overused |
| **HTN** | Compound tasks, ordered methods, conditions, effects | Guerrilla's in-house tools; none public of note | It is close to programming: order, preconditions, effects |
| **GOAP** | Goals, Action Sets, preconditions and effects in symbols | F.E.A.R.'s GDBEdit tables | Predicting the plan the search will find. "Why didn't it do X?" is hard to answer |
| **HFSM** | States and arrows | NodeCanvas FSM, with sub-FSM and sub-BT states; `bevy_gearbox`'s statechart editor | Arrows multiply as states grow ("transition overload", ch. 4) |
| **Creatures** | A genome | The Creatures Genetics Kit | Not authorable as behaviour at all |

**The best authoring evidence points the same way as product fit.** Small utility choices nested in a readable
structure keep each choice to 5–8 options, which answers Isla's tuning warning. Presets answer Lewis's. A small
library of approved presets, like Halo 2's styles, is what a non-programmer actually picks from.

## 5. What "more advanced than behaviour trees" usually means

1. **Utility selection inside the tree.** Merrill's chapter, Behavior Designer Pro, NodeCanvas, and StateTree's utility
   selectors (documented for UE 5.5). The tree still gives structure; scores replace fixed priority where context matters.
2. **StateTree as Epic's successor to BTs.** States and transitions from state machines, selectors from BTs, and
   utility selection in the 5.5 docs. Used with Mass for crowds. Forum benchmarks disagree on whether it is faster than BT; treat them as
   anecdotes.
3. **Planners: GOAP and HTN.** "Advanced" in autonomy: the AI composes plans no one wrote. Studios that tried them often
   moved back for control (Merrill; ch. 4's "loose cannons").

**For this product the answer is 1 and 2 together.** The Pet's appeal is being readable and steerable; a planner's
novelty would cost the visible "why" and make user editing harder. Planning can still come later as one node kind
(a "plan" node for a boss) without changing the rest.

## 6. Crates for Bevy 0.20 (checked 2026-11-06)

| Crate | Kind | Latest | Bevy | Status | Data-loadable tree |
|---|---|---|---|---|---|
| [`big-brain`](https://crates.io/crates/big-brain) | Utility | 0.22.0, 2024-11-30 | 0.15; Codeberg main 0.17 | GitHub archived 2025-10-07 (see simulation-architecture.md) | No: scorers are Rust components |
| [`bevy_observed_utility`](https://crates.io/crates/bevy_observed_utility) | Utility | 0.2.0, 2025-03-05 | 0.15 | Quiet | No |
| [`bevy_behave`](https://github.com/RJ/bevy_behave) | BT | 0.5.0, 2026-02-06 | 0.18; main 0.6.0 on 0.19, pushed 2026-07-07 | Active, no 0.20 branch or PR seen | Not yet: README says loading from an asset "in theory", "PRs welcome" |
| [`bevior_tree`](https://github.com/hyranno/bevior_tree) | BT | 0.12.0, 2026-10-15 | **0.20** | Active, 61 stars | Not verified |
| [`bonsai-bt`](https://github.com/Sollimann/bonsai) | BT, engine-free | 0.14.0, 2026-09-22 | None needed | Active, ~1,100 stars | **Yes**: `Behavior` derives serde behind a feature. No utility selector |
| [`seldom_state`](https://github.com/Seldom-SE/seldom_state) | State machine | 0.16.0, 2026-04-02 | 0.18 | Active | No |
| [`moonshine-behavior`](https://github.com/Zeenobit/moonshine_behavior) | Stack state machine | 0.6.0, 2026-10-11 | **0.20** | Active | No: states are Rust types |
| [`bevy_gearbox`](https://github.com/DEMIURGE-studio/bevy_gearbox) | Statecharts, SCXML rules | 0.9.0, 2026-09-09 | 0.19 | Very active, pushed 2026-11-05; no 0.20 branch or PR seen | **Yes**, as `bsn!` scenes, with a live **visual editor** over Bevy Remote Protocol |
| [`dogoap`](https://crates.io/crates/dogoap) / `bevy_dogoap` | GOAP | 0.5.0, 2025-05-21 | `bevy_reflect` 0.16 / Bevy 0.16 | Quiet | Not verified |
| [`bevy_htn`](https://crates.io/crates/bevy_htn) | HTN | 0.0.1, 2025-03-26 | 0.15 | Experimental, 759 downloads | — |
| [`egui-snarl`](https://github.com/zakarumych/egui-snarl) + [`bevy_egui`](https://github.com/vladbat00/bevy_egui) | Node-canvas UI | 0.12.0 / 0.43.1 | `bevy_egui` 0.43.1 on **0.20**; both on egui 0.36 | Active | It is the editor canvas, not a Mind |

**Verdict: own the Mind in `pet-core`.** Only `bevior_tree` and `moonshine-behavior` support 0.20, and neither loads
trees from Pack data or scores by utility. `bonsai-bt` loads data and needs no Bevy, but would need a utility selector
added, and its node set isn't ours. The evaluator recommended below is roughly 600–1,000 lines of plain Rust with no
IO, which is easy to test Given-When-Then. **Inference.** Watch `bevy_gearbox`: its editor is the closest thing to what
the owner wants, but it is statecharts without utility, and on 0.19.

## 7. Recommendation

### Runtime model: a state tree whose choices are utility scores

Four node kinds, all in `pet-core`, evaluated by one recursive function over a `match`:

| Node kind | Picks | Used by |
|---|---|---|
| `utility` | Highest Infinite Axis score among options whose conditions pass; ties in list order; a small bonus for the running option (Lewis) | The Pet; elites; each boss phase |
| `priority` | First child whose yes/no conditions pass (Halo 2) | Reflex enemies; "must happen" overrides |
| `sequence` | Children in order; fails if one fails | Multi-step Actions such as "walk to edge, sit, sleep" |
| `states` | One active child; switches on transitions with conditions | Pet modes (day, night, focus); boss phases |

Leaves are Actions, exactly as drafted in architecture.js. Options can name Actions by id or **by tag**, so a Form's or
a Pack's new Action joins every choice that lists its tag. Every decision returns a `Why`: the node path, the winner,
its score, and its top two factors with the causes memory gave them.

**The runtime shapes, reserved now** (a proposal; architecture.js isn't edited):

```rust
enum MindNode {                               // in MindDef, read-only, from the Pack
    Utility  { options: Options },            // Options = ids and/or tags
    Priority { children: Vec<NodeId> },
    Sequence { children: Vec<NodeId> },
    States   { children: Vec<NodeId>, transitions: Vec<Transition> },
}
struct MindDef { nodes: Vec<MindNode>, root: NodeId }   // ActionDef gains `tags: Vec<TagId>`
struct MindState {                            // per Actor, changing; replaces `current` in "Pet mind"
    path: SmallVec<[(NodeId, u8); 4]>,        // node and active child per level; the MVP fills one level
    started_at: Tick, next_decision: Tick, last_why: Why,
}
```

The running step of a `sequence`, the active child of `states` and the running-option bonus all read `path`, so the MVP
already has the shape they need. Isla's per-Actor state pooled by tree depth is the precedent. The three Mind
components in architecture.js (`ReflexMind`, `UtilityMind`, `BossPhases`) would become one `MindDef` format with a
`MindState` per Actor; a Reflex enemy's def simply holds only `priority` nodes, and can run in the parallel system.

**The closed vocabulary the app supplies in Rust** (a new kind always needs Rust, which keeps "no scripting"):

- **Inputs:** `drive(name)`, `trait(name)`, the memory queries from pet-memory.md (`mood.valence`, `opinion(subject)`,
  `familiarity`, `since_last(kind|action)`, `recent_count`, `rhythm`), context (`cursor_distance`, `cursor_speed`,
  `owner_idle_for`, `time_of_day`, `activity_level`), own state (`health`, `time_in_action`, `cooldown_ready`), and
  per-target inputs for Encounters (`distance_to_target`, `target_health`).
- **Curves:** linear, polynomial, logistic, logit, normal, sine (Curvature's set), offered first as named presets.
- **Conditions:** an input compared with a constant, combined by all / any.
- **Effects:** the existing fixed list: `Move`, `Emote`, `Attack`; more only by adding Rust.

**Cadence.** Decide on events (a Situation ends, a MemoryEvent, the Action finishes) and on the `next_decision` timer.
Stagger utility Minds in Encounters across ticks; Reflex Minds run every tick in parallel, as architecture.js plans.

### Editable data format: a flat node list in the Pack

Flat lists with string ids, not deeply nested tables, so an editor can round-trip and diff them cleanly. Ids are
strings in the file and become `u16` at load, like `ActionId`.

```toml
[mind]
format = 1                    # mind_format version; the loader migrates older ones
root = "alive"

[[mind.nodes]]
id = "alive"
kind = "utility"              # the MVP's whole Mind: one node
options = { tags = ["idle", "move", "rest", "play"] }   # every Action of the current Form with one of these tags

[actions.chase_cursor]        # the drafted ActionDef, plus tags
tags = ["play"]
weight = 1.0
considerations = [
  { input = "trait.playful", curve = "rising" },                             # a preset
  { input = { memory = "opinion", subject = "cursor" }, bookends = [-1, 1], curve = "logistic" },
  { input = "since_last.chase_cursor", bookends = ["0s", "2m"], curve = "rising" },
]
```

A boss phase or Reflex enemy later is only new rows:

```toml
[[mind.nodes]]
id = "boss"
kind = "states"
children = ["phase1", "phase2"]
transitions = [{ from = "phase1", to = "phase2", when = { input = "health", below = 0.5 } }]
```

**Editor layout lives in a sidecar**, `mind.layout.toml`, holding node positions and collapsed state. The runtime and
`pet-pack validate` ignore it, so the validator's "unknown fields" rule (§7.3) never trips on an editor save.

**The validator adds Mind checks:** no cycles; every id exists; each Form can reach at least one Action; every tag
named by a choice matches at least one Action in some Form; no node names a Situation.

### How this keeps the MVP's Mind editable without a rewrite

The MVP ships the format above with one `utility` node, the `MindState` path and the `Why` record, and writes the
evaluator as the recursive walker from the start, with `utility` as its only arm. Adding `priority`, `sequence` and `states` later adds
arms and Given-When-Then tests; Packs written for the MVP still load, because `format = 1` already describes them.

## 8. Minimum editor versus full editor

**Minimum: a form, not a canvas.** Folded into the §8.2 dev balancing panel:

- a list of the current Form's Actions, each with its weight, tags and on/off switch;
- per consideration: an input drop-down, a curve preset drop-down, bookends, and a small plot of the curve;
- a live **"why" inspector**: every option's score as a bar, the winner, its top factors and their memory causes;
- a "what if" row: set energy to 0.2 and night, and see which Action would win.

It edits the one `utility` node the MVP has, which is all there is to edit. **Inference:** a few days' work with
`bevy_egui`.

**Full: a node canvas**, built with `egui-snarl`:

- the state tree as boxes and wires, with node kinds from a palette and Actions dragged in from the Pack;
- live highlighting of the active path and each option's score, as Unreal's BT debugger does;
- the curve editor with free editing hidden behind presets, as Lewis advises;
- a scenario runner that replays a recorded hour of MemoryEvents through an edited Mind and lists where it decided
  differently;
- validation errors shown on the node that caused them.

## 9. Decisions for the owner

1. **Who edits what?** (a) Users edit the Pack's `mind` table directly, so a user is a Pack author. (b) Users edit a
   per-Character override file in `%LOCALAPPDATA%` that patches weights and considerations by id, so a Pack update
   doesn't wipe their changes. I'd pick (b) for users and (a) for Pack authors, with the same editor.
2. **Editor inside the app or a separate program?** The Overlay is click-through and transparent, and the app has a
   150 MB RAM budget. A separate `pet-mind-editor` that loads the same Pack and runs the same `pet-core` evaluator
   avoids both, at the cost of live editing needing a link to the running Pet. I'd pick a separate program, talking to
   the Pet over a local channel only in dev builds.
3. **How far can users go?** Weights and presets only, or new nodes and free curves too? Halo 2 kept styles to an
   approved library because they were "powerful and dangerous".
4. **Is a user-edited Pet still "alive"?** A user could make it do one thing for ever. Should the validator demand some
   variety, such as a `since_last` consideration on every Action?
5. **Should the "why" be shown to everyone**, or only in the editor and dev panel? This ties into OQ-2 and
   pet-memory.md decision 7.
6. **Does the MVP need `states` for day and night**, or is `time_of_day` as a consideration enough? I'd use the
   consideration and add `states` with bosses.
7. **Reflex format.** Use the same `MindDef` with only `priority` nodes, as §7 proposes, or keep the separate
   "state table" architecture.js drafts? One format means one editor; I'd pick one format.
