// The architecture model. Edit this file and reload index.html.
//
//   version <n>                                     Bump for each feedback round. Red marks belong to a version.
//   title "<text>"                                  Before any `view`, the model's title. After one, that view's title.
//   view <id> "<label>"                             Optional. Starts a tab; everything until the next `view` belongs to it.
//   layer <id> "<label>"                            A horizontal band. Starts a new layer; file order is top to bottom.
//   crate <id> "<label>"                            A box around the nodes that follow it. Belongs to the current layer.
//   node <id> "<label>" <status> @<col>,<row> "<what it does, one line>" [proposed]
//     field "<text>"                                Optional, indented under a node: one monospace line of a data shape.
//   edge <from> -> <to> "<what travels>" [proposed]
//
// Every node is placed by hand on one global grid, so an edit moves only what it touches. Layers and crates are
// drawn around their nodes. `edge` takes node ids or crate ids.
//
// A model with no `view` line is a single view. Once any `view` is used, all layers, crates, nodes and edges must sit
// under one. Each view has its own grid, legend, title and red marks. Node, crate and layer ids need only be unique
// within a view, and an edge joins ids in its own view. A view id is its URL hash: index.html#<id>.
//
// Rows and columns grow to their tallest and widest box. A `field` line widens its column until it fits, up to about
// 50 characters; longer lines wrap. A node with no fields draws no field list.
//
// <status> is the roadmap item that delivers the node: built, item3, item4, item5, item6, item7, or track1 to track7.
// `proposed` means not agreed yet, and draws dashed. A line starting with # is a comment.

window.ARCH = String.raw`
version 3
title "Desktop Pet: target architecture, draft 3"

view systems "Systems"
title "Systems: pure rules in pet-core, ECS wiring in pet-sim, a read-only renderer in pet-app"

layer tracks "Later tracks (seams only)"
  node productivity "Productivity" track4 @1,0 "Focus timer and reminders, delivered by the Pet"
  node ai "AI companion" track6 @2,0 "Talks with the owner"

layer core "Domain core: pure rules, no Bevy, no IO"
  crate pet-core "pet-core"
    node interactions "Interaction rules" item5 @0,1 "Pointer input becomes poke, pet, throw, play"
    node behaviour "Behaviour engine" item4 @1,1 "Drives, utility AI, memory, traits; Reflex tables for Enemies"
    node xp "XP engine" item4 @2,1 "Data-driven sources and level curve"
    node form-graph "Form graph" item4 @3,1 "Evolutions, switchable Forms, transformations"
    node pack-def "Pack definition" item3 @4,1 "Forms with their Actions and one default, Frame durations, Anchor, Body; no pixels"
    node shell-rules "Shell rules" built @0,2 "Presence, fullscreen rule, Hit mask, Relaunch limit"
    node physics "Physics rules" item4 @1,2 "Motion, Ground plane, edges, bounce, swept tests"
    node combat "Combat rules" track3 @2,2 "Hit tests, damage, knockback, teams"
    node situations "Situations" item4 @3,2 "A fixed list the simulation causes itself: held, falling, landing; each Form maps them to its own Animations"

layer sim-layer "Simulation, no rendering"
  crate pet-sim "pet-sim"
    node bridge "Input bridge" item4 @0,3 "Takes time, cursor, idle and pointer into the tick"
    node schedule "Tick schedule" item4 @1,3 "Fixed phases; Companion or Encounter; frozen while Hidden"
    node world "World" item4 @2,3 "Core types stored as components; spawn, despawn, SpatialGrid"
    node clock "Animation clock" item3 @4,3 "Current Frame from per-Frame durations"

layer app-layer "The program"
  crate pet-app "pet-app"
    node overlay "Overlay + startup" built @0,4 "Transparent window, applies Hidden, Relaunch"
    node persistence "Persistence" item6 @1,4 "Own versioned save types and migrations; Progress per Character; backups"
    node settings "Settings" item6 @2,4 "An ordinary window, only while open: scale, monitor, activity, hotkey, sound"
    node renderer "Renderer" item3 @3,4 "Draws the current Frame at the Anchor, integer scale, drop shadow, Emotes"
    node audio "Audio" item6 @4,4 "Optional SFX, off by default"
  crate pet-content "pet-content"
    node loader "Pack loader" item3 @5,4 "Reads TOML + PNGs, validates and reports every error, builds the Definition and pixels"
      field "Hit masks from image transparency"
      field "reused by: pet-pack validate <folder>"

layer os "Operating system"
  crate pet-platform "pet-platform"
    node shell "Shell thread" built @0,5 "Tray, hotkey, autostart, fullscreen watching"
    node multi-monitor "Multi-monitor + other OSes" track7 @1,5 "Roaming, window-aware movement, new OS folders"
    node sensors "Sensors" item4 @2,5 "Global cursor position and idle time only"
    node hit-thread "Hit-test thread" built @3,5 "8 ms poll, flips click-through"
    node watcher "Pack watcher" item3 @5,5 "Tells the app when a Pack's files change; event-driven, no polling"

layer content "Content on disk"
  node art-pipeline "Art & AI pipeline" track5 @3,6 "Produces Packs that meet the art bar"
  crate packs-folder "Packs folder: %LOCALAPPDATA%\DesktopPet\packs"
    node placeholder "Placeholder Pack" item3 @4,6 "The stand-in dragon, as a real Pack"
    node dragon "Dragon Pack" item7 @5,6 "Egg, hatch sequence, hatchling, Emote library"

# Inside the core
edge interactions -> behaviour "touches for memory"
edge interactions -> xp "XP events"
edge xp -> form-graph "level"
edge form-graph -> behaviour "current Form: Actions, modifiers"
edge behaviour -> physics "movement intent"
edge pack-def -> form-graph "Forms, Actions"
edge physics -> situations "held, falling, landing"

# pet-sim wires the rules together
edge bridge -> schedule "time, inputs"
edge schedule -> pet-core "calls the pure rules"
edge pack-def -> clock "Frame durations"
edge world -> renderer "components + messages, read only"
edge clock -> renderer "current Frame"
edge world -> audio "sound messages" proposed
edge persistence -> world "Progress, load + save"

# Packs
edge packs-folder -> loader "TOML + PNGs"
edge loader -> pack-def "builds the Pack Definition"
edge loader -> world "Definition, shared, read only"
edge loader -> renderer "pixels, Emote art"
edge packs-folder -> watcher "file changes"
edge watcher -> loader "reload; on errors keep the old Pack; else PackReloaded"
edge art-pipeline -> packs-folder "generated Packs"

# Program and OS
edge shell-rules -> overlay "Presence"
edge shell -> overlay "ShellEvent + wake-up"
edge sensors -> bridge "cursor, idle time"
edge hit-thread -> bridge "pointer on Pet pixels"
edge renderer -> hit-thread "Hit target"
edge settings -> renderer "scale"
edge settings -> shell "hotkey, autostart"

# Seams for the later tracks
edge productivity -> pet-core "focus sessions"
edge ai -> pet-core "chat, mood"
edge multi-monitor -> sensors "monitor + window geometry"


view runtime "Runtime"
title "Runtime: one tick, left to right, in fixed phases that talk by messages"

layer edges "Off the tick: real events"
  node shell "Shell thread" built @0,0 "OS events: tray, hotkey, fullscreen; wakes the engine"
  node hit "Hit-test thread" built @1,0 "8 ms poll; flips click-through"
  node ai "AI replies" track6 @3,0 "Cloud call off the tick; the reply arrives later"
  node save "Save writes" item6 @5,0 "On ProgressChanged, on a background thread: temporary file, then rename"

layer main "Main thread: always on, light"
  crate sim "pet-sim: thin systems calling the pure rules in pet-core, no rendering"
    node sense "Sense" item4 @0,1 "Takes time and inputs, once a tick"
      field "drain_shell_events"
      field "read_cursor_and_idle"
      field "read_pointer"
    node interpret "Interpret" item5 @1,1 "Raw input becomes gestures"
      field "recognise_gestures"
      field "count_poke_combo"
      field "pick_under_pointer"
    node react "React" item4 @2,1 "Events change drives, memory, XP, Forms"
      field "update_drives"
      field "remember"
      field "award_xp"
      field "check_evolutions"
    node decide "Decide" item4 @3,1 "Scores Actions on a slow beat, or at once when woken"
      field "pick_action"
      field "pick_emote"
    node act "Act" item4 @4,1 "Runs the Action, then moves Actors, in this order"
      field "run_action"
      field "advance_animation_clocks"
      field "move_actors"
      field "collide_with_world"
    node resolve "Resolve" track3 @5,1 "Applies this tick's Hits, then removes the dead"
      field "apply_hits"
      field "despawn_dead"
  crate app "pet-app"
    node present "Present" item3 @6,1 "Reads components and messages; nothing flows back"
      field "draw_current_frames"
      field "draw_shadow"
      field "show_emotes"
      field "publish_hit_targets"

layer encounter "Encounters only: heavy, all cores"
  crate sim-encounter "pet-sim: off in Companion, costing almost nothing"
    node waves "Waves" track3 @2,2 "Spawns Enemies as an Encounter runs"
      field "spawn_waves"
    node minds "Minds" track3 @3,2 "Many cheap Reflex Minds in parallel; smart ones staggered"
      field "run_reflex_minds (parallel)"
      field "run_utility_minds"
    node grid-hits "Grid + hits" track3 @4,2 "The last two steps of Act"
      field "rebuild_spatial_grid"
      field "detect_hits (parallel)"

layer control "What switches the tick"
  node modes "Mode" track3 @2,3 "Companion or Encounter; one condition switches encounter systems"
  node hidden "Hidden" built @3,3 "No updates run, so the tick freezes; waking must not burst"
  node pool "Compute pool" track3 @4,3 "Worker threads, used only by parallel queries"
  node encounter-clicks "Encounter clicks" track3 @5,3 "While an Encounter runs, the Overlay catches every click on the Home monitor, taskbar too; Escape forfeits"

# Off the tick, into the tick
edge shell -> sense "ShellEvent (channel)"
edge hit -> sense "pointer on Pet pixels"
edge ai -> sense "AI reply (channel)"

# One tick
edge sense -> interpret "PointerEvent"
edge interpret -> react "PetPoked, PetStroked"
edge react -> decide "DecideNow"
edge decide -> act "ActionChosen"
edge act -> react "Landed (next tick)"
edge act -> grid-hits "positions after collision"
edge grid-hits -> resolve "Hit"
edge interpret -> resolve "EnemyClicked"
edge act -> present "Actor components, current Frame"
edge resolve -> present "ActorHurt, ActorDied"
edge decide -> present "ShowEmote, HideEmote"

# Out of the tick
edge present -> hit "Hit target"
edge react -> save "ProgressChanged"

# Encounter
edge minds -> act "movement intent"
edge waves -> minds "new Enemies"

# Control
edge modes -> sim-encounter "runs only in Encounter"
edge pool -> sim-encounter "parallel queries"
edge hidden -> sim "stops every update"
edge encounter-clicks -> hit "catch every click"


view physics "Physics & hits"
title "Physics & hits: what Act and Resolve do each tick, and which shape does which job"

layer tick "Per tick, in order"
  crate act-steps "Act"
    node clock "Animation clock" item3 @0,0 "Current Frame from the Pack's per-Frame durations; the renderer only draws it"
      field "advance_animation_clocks"
      field "reads: CurrentAction, Frame durations"
      field "writes: AnimationClock"
    node move "Move" item4 @1,0 "Integrates forces: gravity, steering intent, knockback"
      field "move_actors"
      field "reads: movement intent, Knockback"
      field "writes: Motion"
      field "no gravity in Fly or Hover mode"
    node world "World collision" item4 @2,0 "Ground plane, screen edges, bounce"
      field "collide_with_world"
      field "reads: Body, Ground plane, Home monitor"
      field "writes: Motion (clamp, bounce)"
      field "emits: Landed"
    node grid "Grid rebuild" track3 @3,0 "Counting sort, single-threaded"
      field "rebuild_spatial_grid"
      field "reads: Motion, HurtBox"
      field "writes: SpatialGrid"
    node hits "Hit detection" track3 @4,0 "Broad phase: the grid. Narrow phase: rectangle overlap"
      field "detect_hits (parallel query)"
      field "reads: StrikeBox, HurtBox, Teams, SpatialGrid"
      field "fast movers: segment vs target box grown by mover"
      field "per-thread results merged, sorted by entity id"
      field "emits: Hit"
  crate resolve-step "Resolve"
    node resolve "Resolve" track3 @5,0 "Applies every Hit of this tick, in entity id order"
      field "apply_hits, despawn_dead"
      field "reads: Hit"
      field "writes: Health, Knockback; despawns"
      field "emits: ActorHurt, ActorDied"

layer shapes "Three shapes, three jobs"
  node hit-mask "Hit mask" built @0,1 "Pixels of the current Frame; routes clicks, nothing else"
    field "Hit-test thread: one target today"
    field "rectangle check first, then pixels"
    field "Encounters may catch every click (Runtime)"
  node body "Body" item4 @2,1 "One rectangle per Form, relative to the Anchor; the world only"
    field "Body { rect: Rect }"
    field "an Action may override it"
  node boxes "Hurt box + Strike box" track3 @4,1 "Rectangles for combat, never for the world or clicks"
    field "HurtBox: where it can be hit"
    field "StrikeBox: live only in the attack's"
    field "  active window of Frames, from the Pack"
    field "each target hit at most once per attack"

layer strikers "Who strikes"
  node click "Owner's click" item5 @0,2 "Picked through the grid, then pixels of the current Frame, topmost first"
    field "pick_under_pointer, in Interpret"
    field "emits: EnemyClicked or PetPoked"
  node teams "Teams" track3 @1,2 "A bitmask decides who can hit whom"
    field "Teams(u8): PET | OWNER | ENEMY"
    field "hits when strike.teams & target.team != 0"
  node melee "Melee Actor" track3 @3,2 "A Strike box during the attack's active window"
  node projectile "Projectile" track3 @4,2 "Motion, Strike box, lifetime, pierce, team; no Mind, no health, so not an Actor"
  node thrown "Thrown Pet" track3 @5,2 "A Strike box while its speed is above a threshold (throw-as-attack)"

layer limits "Modes and limits"
  node companion "In Companion" item4 @1,3 "Move and World collision for the Pet only; grid and hits are off"
  node no-engine "No physics engine" item4 @3,3 "No rotations, stacking or joints, so plain rules are enough"

# Order inside the tick
edge clock -> move "then"
edge move -> world "then"
edge world -> grid "then"
edge grid -> hits "then"
edge hits -> resolve "Hit"

# Which shape feeds which step
edge clock -> hit-mask "current Frame's mask"
edge clock -> boxes "active window"
edge body -> world "Body rect"
edge boxes -> hits "rectangles"
edge hit-mask -> click "pixel test"
edge grid -> click "candidates under the pointer"

# Who strikes
edge teams -> hits "who can hit whom"
edge melee -> boxes "owns a Strike box"
edge projectile -> boxes "carries a Strike box"
edge thrown -> boxes "Strike box above a speed"

# Companion
edge companion -> move "Pet only"
edge companion -> world "Pet only"


view domain "Domain model"
title "Domain model: the things themselves, not the crates"

layer definitions "Read-only, from the Pack"
  crate defs "Definitions"
    node character "Character" item3 @0,0 "An identity, such as the dragon. Traits are fixed per Character"
    node form-graph "FormGraph" item4 @1,0 "How Forms connect"
      field "Evolution: permanent"
      field "Transformation: temporary"
    node form "Form" item3 @2,0 "One stage: its art, its Actions with one default, its Body, trait changes"
    node action-def "ActionDef" item4 @3,0 "What an Action scores on, its timing, its effects"
      field "effects: Move, Emote, Attack"
    node enemy-def "EnemyDef" track3 @4,0 "One kind of Enemy: art, stats, which Mind"
    node emote-def "EmoteDef" item3 @5,0 "Pack-level art shared by every Form; one-shot, or looping until hidden"

layer world "Running: exists only while the app runs"
  crate running "Running world"
    node situation "Situation" item4 @0,1 "Something the simulation causes itself, such as falling; a Form maps it to an Animation"
    node pet "Pet" item4 @1,1 "The one Character on screen"
    node actor "Actor" item4 @2,1 "Anything that moves and acts: Anchor position, Action, facing"
    node mind "Mind" item4 @3,1 "What decides the next Action of an Actor"
      field "Reflex: basic Enemy"
      field "Utility: the Pet, elites"
      field "Utility + phases: boss"
    node body "Body" item4 @4,1 "One rectangle per Form, an Action may override it; never for clicks or combat"

layer saved "Saved, one record per Character"
  crate progress "Progress"
    node form-progress "FormProgress" item4 @0,2 "Current Form. Unlocked Forms only grow"
    node ledger "Ledger" item4 @1,2 "XP never shrinks, so Level never drops"
    node pet-memory "PetMemory" item4 @2,2 "What it remembers of recent interactions"

layer seams "Later tracks plug in here"
  crate later "Encounter and focus"
    node focus "FocusSession" track4 @1,3 "No Evolution and no Encounter during one"
    node encounter "Encounter" track3 @2,3 "One timed fight; skipping or failing costs nothing" proposed
      field "Wave: a group of Enemies"
    node enemy "Enemy" track3 @3,3 "An Actor from an EnemyDef; lasts one Encounter" proposed
    node item-relic "Item / Relic" track3 @4,3 "Loot; can meet an Evolution condition" proposed
    node projectile "Projectile" track3 @5,3 "Fired by an Attack; moves and strikes, but is not an Actor"

edge character -> form-graph "has one"
edge form-graph -> form "connects"
edge form -> action-def "declares"
edge form -> situation "maps to its Animations"
edge character -> emote-def "has a library of"
edge pet -> character "is one"
edge pet -> form "is in one"
edge pet -> actor "is an"
edge pet -> progress "has"
edge actor -> mind "is driven by"
edge actor -> body "has a"
edge mind -> action-def "scores"
edge ledger -> form-progress "Level opens Evolutions"
edge form-progress -> form-graph "points into"
edge focus -> form-progress "blocks Evolution"
edge focus -> encounter "blocks" proposed
edge encounter -> enemy "spawns Waves of" proposed
edge enemy -> actor "is an" proposed
edge enemy -> enemy-def "built from" proposed
edge enemy -> item-relic "drops" proposed
edge action-def -> projectile "an Attack can fire"


view data "Data structures"
title "Data structures: the real shapes, Definitions above, changing data below"

layer definitions "Read-only, shared"
  node ids "Ids" item3 @0,0 "Small numbers, made from Pack names at load"
    field "struct FormId(u16)"
    field "struct ActionId(u16)"
    field "struct EmoteId(u16)"
    field "struct ItemId(u16)"
    field "// hot loops never compare text"
  node form-graph "FormGraph" item4 @1,0 "An array; an id is an index"
    field "forms: Vec<FormDef>"
    field "evolutions: Vec<Evolution>"
    field "transformations: Vec<Transformation>"
    field "Evolution { from, to, when: Vec<Condition> }"
    field "Condition = Level(u32) | Item(ItemId)"
    field "  | Relic(ItemId) | Event(EventId)"
    field "Transformation { form, trigger, duration, cooldown }"
  node action-def "ActionDef" item4 @2,0 "Infinite Axis utility shape" proposed
    field "id: ActionId"
    field "weight: f32"
    field "considerations: Vec<Consideration>"
    field "timing: Timing"
    field "effects: Vec<Effect>  // fixed list, no scripts"
    field "Effect = Move | Emote(EmoteId) | Attack(AttackDef)"
    field "Consideration { input, bookends, curve }"
    field "Curve { kind, slope, exponent, x_shift, y_shift }"
  node xp-source "XpSource" item4 @3,0 "One per kind of XP, as data"
    field "id: XpSourceId"
    field "base: u32"
    field "multiplier: f32"
    field "cap: Option<u32>"
    field "enabled: bool"
    field "level_curve: Vec<u64>  // shared by all sources"
  node form-def "FormDef" item3 @4,0 "One Form, read-only, from the Pack"
    field "actions: Vec<ActionId>"
    field "default_action: ActionId"
    field "animations: Vec<AnimationDef>"
    field "anchor: Point"
    field "body: Rect"
    field "body_overrides: Vec<(ActionId, Rect)>"
    field "// missing Animation: play the default, log once"
    field "situations: [AnimationId; N]  // proposed"
  node emote-def "EmoteDef" item3 @5,0 "Pack-level, read-only"
    field "animation: AnimationId"
    field "mode: OneShot | Looping"
    field "// Looping stays until HideEmote"

layer changing "Progress: saved per Character"
  node form-progress "FormProgress" item4 @1,1 "Unlocked only grows"
    field "current: FormId"
    field "unlocked: BTreeSet<FormId>"
    field "transformation: Option<Active>"
    field "cooldowns: Vec<Cooldown>"
  node pet-state "Pet mind" item4 @2,1 "Drives run; memory is saved" proposed
    field "drives: [f32; DRIVE_COUNT]"
    field "traits: [f32; TRAIT_COUNT]"
    field "memory: RingBuffer<Recollection>"
    field "current: ActionInstance"
    field "next_decision: Tick"
  node ledger "Ledger" item4 @3,1 "Only ever adds"
    field "total: u64"
    field "by_source: Vec<u64>"

layer running "Running: Actor parts and messages"
  node grid "SpatialGrid" track3 @0,2 "Finds nearby Actors, on the line or in the air" proposed
    field "cell_size: f32"
    field "starts: Vec<u32>"
    field "entries: Vec<(Entity, Rect)>"
    field "// rebuilt each tick by counting sort"
  node actor "Actor components" item4 @1,2 "Small pieces, so a system touches only what it needs" proposed
    field "Motion { position: Vec2, velocity: Vec2,"
    field "  mode: MovementMode }  // position = Anchor"
    field "Body { rect: Rect }  // Anchor-relative"
    field "Facing, Health, Teams"
    field "CurrentAction { id: ActionId, started_at: Tick }"
    field "AnimationClock { animation, frame: u16,"
    field "  elapsed: Ms }"
  node combat "Combat components" track3 @2,2 "Only on things that fight" proposed
    field "HurtBox { rect: Rect }"
    field "StrikeBox { rect: Rect, window: FrameRange,"
    field "  damage: u32, knockback: Vec2, teams: Teams,"
    field "  already_hit: Vec<Entity> }"
    field "Projectile { lifetime: Ms, pierce: u8 }"
    field "Teams(u8)  // bitmask: PET | OWNER | ENEMY"
    field "// a Projectile: Motion + StrikeBox, no Mind"
  node mind "Mind components" track3 @3,2 "One component per kind of intelligence, one system each" proposed
    field "ReflexMind { table: StateTableId, state: u8 }"
    field "UtilityMind { brain: Brain }  // Pet, elites"
    field "BossPhases { phase: PhaseId }  // + UtilityMind"
  node messages "Messages" item4 @4,2 "Buffered, read once in a later phase" proposed
    field "Hit { attacker, target, damage, knockback, point }"
    field "Landed { actor }"
    field "ActorHurt { actor }, ActorDied { actor }"
    field "ShowEmote { actor, emote }, HideEmote { actor }"
    field "ProgressChanged, PackReloaded  // proposed"

edge ids -> form-graph "used in every table"
edge form-graph -> form-def "forms: Vec<FormDef>"
edge form-graph -> form-progress "ids point into"
edge action-def -> pet-state "scored against"
edge xp-source -> ledger "awards"
edge ledger -> form-progress "Level meets Conditions"
edge pet-state -> mind "feeds UtilityMind" proposed
edge actor -> mind "an Actor has one" proposed
edge grid -> actor "indexes" proposed
edge actor -> combat "fighters add" proposed
edge combat -> messages "detect_hits writes Hit" proposed
`;
