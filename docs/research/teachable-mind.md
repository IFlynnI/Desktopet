# A teachable Mind: emergent combinations, fast learning, lasting knowledge

**Researched:** 2026-11-06, for the end goal of the Pet's Mind (BEH-02, BEH-04, §8.1, §10.2, Tracks 1–3).
**Purpose:** take [pet-memory.md](pet-memory.md) (design B) and [mind-architecture.md](mind-architecture.md) (a state
tree with utility choices) further. The goal is a Mind with "true understanding of its surroundings / of scenarios, fast
learning and lasting memory capability". Two scenarios are the test. In A, the Pet learns to ask to be thrown and
smashes above an enemy group. In B, it is taught to shoot a thrown ball for a reward. **Revised the same day for two
directions from the owner:**
- Combinations must *emerge*. Actions are defined up front, and how they combine comes from play and the Pet's own
  experiments.
- Structured randomness may be allowed.
- No Action may be added per scenario (later correction). How the Pet *tells* the owner what it wants is a separate
  system: Emotes now, the AI track later. It is out of scope here.

[neural-mind.md](neural-mind.md) already rules out deep networks at runtime, so this doc stays with small tables.

| Mark | Meaning |
|---|---|
| **Read** | I read the full text for this doc. For PDFs I extracted the text myself; for some pages a fetch tool summarised the full text, and I checked the numbers quoted here against it. |
| **Summary** | I found the primary source, but read only its abstract, its listing, or write-ups about it. |
| **Inference** | My reasoning or estimate. Not measured. |

## Bottom line

- **Emergence is affordable here because the Pet only has to learn what the engine can't tell it.** The planner runs
  any one Action forward with the engine's own rules: the arc of a throw, impact speed, AoE damage, bounce. That is the
  mechanics' definition, not a scripted combo. Only **links** the engine can't simulate are learned: "when I'm by your
  cursor during play, you pick me up 4 times in 10", "when you throw the ball and I hit it, you pet me". A planner that searches 3 steps
  ahead chains simulated and learned links into combos nobody wrote down.
- **That only works if the mechanics are generic.** Smash damage must come from fall speed, not from an "if thrown"
  case. Every Action is defined by what it does to shared world features. **No Action is added for a scenario:**
  Scenario A uses only the owner's throw, the existing `follow_cursor` Action and `smash`. Going to the cursor gains
  value from a learned link ("near your cursor → you pick me up → thrown → good smash").
- **Sample counts are per learned link, not per chain:** about 3–10 times each. Learned links compose without the whole
  chain ever being tried. Compare OpenAI's hide-and-seek: forts took about 25 M episodes and box surfing about 380 M,
  model-free, from a reward that wasn't aligned with the behaviour. An object-based model learned Taxi in 529 steps
  (Diuk et al. 2008) where Q-learning took 106,859 (Diuk's dissertation), about 200 times fewer. Both figures come
  via a reimplementation's table.
- **Honest limits:** 2–4-link combos built from existing Actions, learned over hours to days of normal play, are
  achievable. New motor skills, chains of 5 or more learned links, and anything needing thousands of failures (box
  surfing) are not. The Pet "understands" exactly what the features and simulators expose.
- **Randomness: recommend seeded, bounded, labelled exploration.** Exploration is driven by curiosity, based on learning
  progress (Oudeyer), not raw surprise, so the owner's unpredictable petting can't hook it. Only options scoring at
  least half the best can be picked. It is never used during focus, is always shown as ❓, and fades as learning
  plateaus. It is reproducible from a seed kept in Progress, so tests stay exact. Proposed pillar wording is in §7.
- **The reward is the valence MemoryEvents already carry.** Credit goes to what was running 0.3–4 s before (TAMER),
  and a middle-click marker credits it exactly. A reward counts only above the owner's usual rate.
- **Cost is small:** depth 3 and beam 4 come to about 90 nodes and 0.1–0.2 ms a plan, replanned on events and at most
  once a second. That averages about 5 µs a tick, about 0.1 % of the companion headroom. RAM is 20–40 KB. **Inference.**
- **MVP taste:** it learns which Actions you pet it after, by time of day and whether the cursor is near, and its ❓
  experiments show it trying things.

## The picture

```text
 world, owner Gestures ─► Perception: Rust primitives, Pack names and bins ─► features (height, velocity, material, cursor…)
        │                                                                          │
        ▼                                                                          ▼
 Simulators (pet-core rules, one Action ahead)  ◄─┐               Mind tree (Pack) ── utility choice ── considerations
 Learned links: (context, Action) ─► result     ◄─┼─ Planner: depth 3, beam 4 ─► plan_value(first Action) ┘
 Outcome cells: result × context ─► reward      ◄─┘              ▲
        ▲                                      Curiosity (learning progress) ─► seeded experiment ❓
 MemoryEvents (valence) + marker ─► Credit ─► update links, cells, residuals
 every read returns value + cause ──► "why": each link with its evidence, simulated ones marked ⚙
```

**If you're used to web apps:** the simulators are your unit-tested business rules, run as a dry run. Learned links are
analytics about *users*: "after we show the banner, 60 % click". The planner is a query optimiser over both. Curiosity
is a small, labelled experiment budget that shrinks once the metrics stop moving.

## 1. Animal training: what makes learning fast

| Source | What it teaches | Mark |
|---|---|---|
| Blumberg et al., [Integrated Learning for Interactive Synthetic Characters](https://characters.media.mit.edu/Papers/siggraph2002.pdf), SIGGRAPH 2002 | A clicker-trained virtual dog. Dogs treat what "immediately precedes" a significant consequence as causal. The click marks the behaviour and bridges to the treat. Teach the action first, then the cue. Attend only to cues near an action's onset. Keep a reward *rate* over the last few trials. Its stated limit: proximate causes only, not sequences. Its action choice and shaping are probabilistic. | **Read** |
| Lattal, [Delayed reinforcement](https://pmc.ncbi.nlm.nih.gov/articles/PMC2801538), JEAB 2010 | Shaping with a 1 s delay worked within one 1-hour session. At 10 s, one pigeon wasn't shaped after ten sessions. Signalled delays beat unsignalled ones. | **Read** |
| Knox & Stone, [TAMER](https://www.cs.utexas.edu/~pstone/Papers/bib2html-links/AAAIsymp09-knox.pdf), 2009 | Human reward lands on recent behaviour along a gamma curve with **zero credit for 0–0.3 s**. Tetris: 65.9 lines by game 3, where RL baselines took 120 to 3,000 games to reach their peaks. | **Read** |
| Thomaz & Breazeal, [RL with human teachers](https://www.media.mit.edu/publications/reinforcement-learning-with-human-teachers-understanding-how-people-want-to-teach-robots), 2006 | Teachers are strongly positive and use reward to guide what comes next. | Summary |
| Peterson, [Skinner's discovery of shaping](https://pmc.ncbi.nlm.nih.gov/articles/PMC1285014), 2004; Feng et al., [clicker review](https://doi.org/10.1016/j.applanim.2016.05.012), 2016; Garcia & Koelling, [cue to consequence](https://doi.org/10.3758/BF03342209), 1966 | Shaping by approximations; the clicker as a conditioned reinforcer; "preparedness" (one-trial taste aversion, for some pairings only). | Summary |

**Lessons:**
1. Mark the moment, with a marker Gesture.
2. Shape by approximation, through learned residuals.
3. Chain backwards: practise the last link (smash) first.
4. Learn fast by considering few pairings, which is what the Pack's feature list does.
5. Animals mostly learn *proximate* links. That is why the planner, not the learner, does the chaining.

## 2. Game precedents

| Game | What it did | Lesson | Mark |
|---|---|---|---|
| **Black & White** ([Eurogamer](https://www.eurogamer.net/black-and-white-lionhead-peter-moylneux-retrospective-google-deepmind-gemini-ai), [Wikipedia](https://en.wikipedia.org/wiki/Black_%26_White_(video_game)); Evans 2002 chapter not read) | BDI: desires were perceptrons, opinions decision trees. Slap and stroke. It had to learn "from a single piece of player feedback". Even Evans couldn't predict it. | One-shot learning over a few desires works. Unbounded learned structure surprises even its author. | Summary |
| **Petz** ([GDC classic](https://www.gamedeveloper.com/design/gdc-classic-multiple-character-interaction-in-i-petz-i-games)) | Treats lure tricks, a squirt bottle disciplines. "Constrained randomness" keeps personality but varies the behaviour. No stat bars. | Shipped, *bounded* randomness felt alive. The visible reaction is what sells the learning. | **Read** |
| **Nintendogs**, **The Last Guardian**, **Dragon's Dogma** ([Nintendogs](https://en.wikipedia.org/wiki/Nintendogs), [CEDEC 2017 slides](https://docs.google.com/presentation/d/1cYDabTQGYe09w3PyUU4XLrDn2fHy9X5eJIFUjVDt89U/htmlpresent), [pawns](https://news.capcomusa.com/lets/browse/dragons-dogma-pawns-learn-organically-as-you-play)) | A light bulb marks a teachable moment, and 3–4 repetitions bind a trick. Trico attends, then moves, then acts; it was deliberately less obedient, and players read it through its ears. Pawns show enemy knowledge as 0–3 stars. | Show the teachable moment (💡). Attention first. Show learned knowledge as a scale. | Summary |
| **Creatures** (Grand et al. 1996; see pet-memory.md) | A decaying susceptibility lets a delayed reward reach an action. | That is an eligibility trace. | Read for pet-memory.md |
| **Spore** ([IEEE Spectrum](https://spectrum.ieee.org/engineering-spore)) | Procedural rules from body shape; no teachable learning found. | Nothing to borrow. | Summary |

## 3. How combinations emerge from few examples

| Method | What it shows | What the Pet takes | Mark |
|---|---|---|---|
| Drescher, *Made-Up Minds* 1991 ([TR](https://ftp.mirrorservice.org/sites/www.bitsavers.org/pdf/mit/lcs/tr/MIT-LCS-TR-0563.pdf)); Holmes & Isbell, [Schema learning](https://proceedings.nips.cc/paper_files/paper/2004/file/18bb68e2b38e4a8ce7cf4f6b2625768c-Paper.pdf), NIPS 2004 | A **schema** is context → action → result. *Discovery* keeps a result when P(result \| action) / P(result \| no action) > θ, with θ > 1. *Refinement* then adds the context features that make the schema reliable. Schemas chain into plans. Holmes & Isbell's learners reached minimum error within **1,000 steps** on small benchmark POMDPs. | The learned-link store (§5.3), its ratio test, and context refinement. | Holmes & Isbell: Summary, numbers checked in extracted text; Drescher Summary |
| Sutton 1991, [Dyna](https://papersdb.cs.ualberta.ca/~papersdb/view_publication.php?pub_id=500); Diuk, Cohen & Littman 2008, [OO-MDP](https://mlanthology.org/icml/2008/diuk2008icml-object) | Plan over a learned model instead of learning every chain by trial. With objects and relations as the representation, DOORmax learned 5×5 Taxi in **529 steps** and factored Rmax took 1,676 (paper). Rmax took 4,151 and Q-learning 106,859 (Diuk's dissertation). | Plan over simulated plus learned links. Features are about objects (enemy group, ball, owner), not pixels. | Summary (Taxi figures from the [reimplementation's table](https://github.com/borea17/efficient_rl)) |
| Learned planning operators: Yang, Wu & Jiang 2007, [ARMS](https://cse.hkust.edu.hk/~qyang/Docs/2007/armsAIJ.pdf) | Learns STRIPS-style preconditions and effects from observed plans: a learned GOAP. | Links are effects with learned preconditions. The search stays at depth 3, so the "why" stays readable. | Summary |
| Options discovery: McGovern & Barto 2001, [diverse density](https://mlanthology.org/icml/2001/mcgovern2001icml-automatic); Chentanez, Barto & Singh, [Intrinsically motivated RL](https://papers.nips.cc/paper/2552-intrinsically-motivated-reinforcement-learning), NIPS 2004 | Subgoals are states that many successful paths share. In the playroom, curiosity learns skills for the light, the music and the bell, then chains them to make a toy monkey cry out. | A plan that keeps paying off is saved as a **routine**: a named macro shown in the dev panel, which is a discovered option. | Summary |
| Curiosity: Oudeyer, Kaplan & Hafner 2007, [IAC](https://adapt.informatik.hu-berlin.de/pub/papers/ims.pdf); Schmidhuber 2010, [formal theory](https://webcf.waybackmachine.org/web/20240324142949/https://people.idsia.ch/~juergen/ieeecreative.pdf); Pathak et al. 2017, [ICM](https://ar5iv.labs.arxiv.org/html/1705.05363) | IAC rewards **learning progress**, the drop in prediction error within a region. It avoids both the boring and the unlearnable. Its AIBO playground ran for several hours per run, with a random action 35 % of the time (15 % in another run). Schmidhuber also rewards progress, which he calls compression. ICM rewards prediction error only in features the agent can control, to escape the "noisy TV". | Curiosity = learning progress per region of the link store. The owner is a noisy TV, so raw surprise is out. | ICM **Read**; IAC: Summary, numbers checked in extracted text; Schmidhuber Summary |
| Baker et al. 2019, [hide-and-seek](https://ar5iv.labs.arxiv.org/html/1909.07528) | Forts at about 25 M episodes, ramps at about 100 M, ramp defence at about 132 M (31.7 B frames, 34 h), box surfing at about 380 M. The authors blame a reward "not directly aligned" with the behaviours. Count-based curiosity got *worse* once box rotation and velocity were added to the state. | Model-free emergence is out of reach for one Pet. Add physics features such as Materials or release speed to a link only when refinement shows they matter. | **Read** |
| Blundell et al. 2016, [MFEC](https://ar5iv.labs.arxiv.org/html/1606.04460); Pritzel et al. 2017, [NEC](https://ar5iv.labs.arxiv.org/html/1703.01988); Li et al. 2010, [LinUCB](https://arxiv.org/abs/1003.0146); Sutton & Barto, [ch. 12](http://incompleteideas.net/book/the-book-2nd.html); Argall et al. 2009, [LfD survey](https://ri.cmu.edu/publications/a-survey-of-robot-learning-from-demonstration) | Episodic control ("do what worked here") beat DQN early: 54.6 % vs 15.7 % of human median at 10 M frames. MFEC's max-return is risk-seeking in noise. Bandits learn per context. Traces carry credit back in time. Demonstrations seed policies. | Outcome cells hold a mean, not a max. The credit window is a trace in seconds. Owner commands count as demonstrations (Track 3). | MFEC, NEC **Read**; rest Summary |

**What the numbers say for one Pet.** **Inference**, from the figures above and neural-mind.md's 20–100 labelled
moments a day.

| Thing to learn | Experiences needed |
|---|---|
| One link with a reliable cause (physics-like, engine events) | 3–5 co-occurrences pass a ratio test θ = 2 |
| One link through the owner, who obliges about 60 % of the time | 5–10 tries |
| Refining a link with one more context feature | 10–20 more |
| A 3-link combo where 1–2 links are learned and the rest simulated | 5–20 relevant moments; the whole chain never has to be tried |
| A combo that needs a new motor skill, or 5 or more learned links | Not achievable without the owner shaping each link |

## 4. Mechanics that make emergence possible

The Mind can only combine what the mechanics let combine. Rules for Rust and Pack authors:

1. **Effects are quantities from physics, never special cases.** For example, smash damage = 0.02 × impact speed (px/s)
   within 120 px. A throw, a high ledge or a bounce all raise impact speed with no extra code.
2. **Actions are defined by what they do to shared features:** velocity, position, contact, Health, prop state. A named
   outcome like "combo" never appears, and no Action exists only to serve one scenario.
3. **One physics for everything.** Bodies are upright AABBs that never rotate. The Pet, enemies and the ball share
   gravity and per-Material friction and bounciness. Per-Action overrides are data, such as a zero-bounce hero landing
   for smash. The owner throws through a springy grab "string", flicked or swung, so a throw's speed and direction are
   physics outputs the Pet can read like any other. A lesson about one body then carries to others.
4. **Owner Gestures are world events with the same features.** Held, thrown with a velocity, petted. "When I'm by your
   cursor you pick me up" is learned like any other link.
5. **Rewards are about outcomes:** an enemy defeated, petted, a treat. Never about using a combination.
6. **Every rule is a pure `pet-core` function** (PRD §7.2), so the planner can run one Action forward with the same
   code the simulation uses. That is the Pet's "imagination".
7. **The planner's simulator may be cheaper than the real one.** For example, it can ignore air drag and the string's
   spring. A learned **residual** per Action ("my smashes land 40 px ahead") corrects it, and refinement adds a
   feature such as `release_speed` or `contact_material` to that residual's context only if the errors follow it.

## 5. Proposed architecture

### 5.1 Perception: features

| Tier | Defined by | Examples |
|---|---|---|
| Primitives | Rust, a closed list | `height_above_ground`, `velocity`, `descending`, `release_speed`, `contact_material`, `dx_to(selector)`, `count_within(selector, r)`, `centroid(selector)`, `cursor_distance`, `prop_state(tag)`, `since_event(kind)`, `gesture_recent(kind, window)` |
| Named features | Pack data: a primitive, its arguments, named bins | `over_group = dx_to(centroid(enemies within 300))`, bins `near < 60 px, close < 120`; `ball_flying = prop_state(ball) == flying` |
| Simulators | Rust, pure functions from Definitions | `simulate(action, state)`, covering ballistic arc, impact speed, AoE hits, bounce by material; `now_vs_best_ahead(quantity)` along the simulated path |

Links and outcome cells use at most 3 context features of about 4 bins each (≤ 64 cells), which the validator
enforces. Refinement may swap in another Pack-listed feature, such as `contact_material`, when it raises reliability.

### 5.2 Reward and credit (unchanged in substance)

- **Reward = valence × intensity** of MemoryEvents. Petting +0.6, `treat` +0.8, `enemy_defeated` +0.2, `strike_hit`
  +0.05 per target, `marker` 0, which only aims credit.
- **Credit window:** shared among Actions *running at any time* 0.3–4 s before the reward. Each gets the highest value
  a gamma-like curve, peaking near 1 s, takes while it ran. A **marker** (middle click on the Pet) gives all credit to
  the Action live 0.3–1.5 s before it, and bridges any reward in the next 10 s to that Action.
- **Contingency:** each cell keeps the owner's baseline reward rate, and learns `mean − baseline`.
- **No guilt:** time by the cursor without a pick-up, or a withheld reward, counts **0**. It lowers that link's probability only. It never
  touches `opinion(owner)`.

### 5.3 What is learned and stored

```text
Link        { context: u16, step: Action | Situation, result: FeatureChange, yes, tries, base_rate, last: [EpisodeRef; 3] } ~40 B
OutcomeCell { result_or_action, context, n, mean, baseline, last: [EpisodeRef; 3] }                         ~48 B
Residual    { action, context, mean_error_px, n }                                                           ~16 B
Routine     { steps: [Step; ≤4], context, uses, mean_value }   a discovered option                           ~32 B
Rng         { seed: u64, draws: u64 }                                                                         16 B
```

- **Links** are discovered when `P(result | action, ctx) / P(result | no action, ctx) > 2` with yes ≥ 3. The
  probability shown is `(yes + 1)/(tries + 2)`. Refinement tries one Pack-listed feature at a time and keeps it if
  reliability rises by more than 0.2 over at least 10 tries.
- **Outcome cells** say what a *result* is worth in a context, such as `prop_struck | owner_threw_ball → +0.55`. They
  update as `mean += α(outcome − mean)`, with α = 1/n up to n = 4 and 0.25 after. Two speeds and consolidation on Sleep
  work as in pet-memory.md.
- **Routines:** a plan that is chosen and pays off 5 times becomes a named macro, so it is reused without searching.

### 5.4 The planner

At each decision, a beam search of depth ≤ 3, beam 4, over eligible Actions (those whose tree conditions pass, about
10). Each step uses `simulate()` where the engine can say what happens, and the best-matching learned link where it
can't (the owner, enemy reactions). Between Pet Actions the planner may insert up to 2 **world steps**: a Situation the
Pet would be in, such as held, whose outcome comes from links keyed on that Situation. World steps don't count
towards depth 3. A link with no data in this context blends the nearest filled cell with the Pack's `unknown_link`
guess (0.3): `p = 0.5 × p_nearest + 0.5 × 0.3`. With no cell at all it uses 0.3, and the "why" marks it as a guess. Plan value = Σ probability × outcome-cell value, discounted 10 % per second of plan time. The best
plan's value reaches scoring as the consideration `plan_value(first Action)`, bookends `[0, 0.5]` → ×1.0 to ×2.0.
Drives, traits, memory and Pack `limits` therefore still shape the choice. During a fall, the `air` node replans every
tick. Any air Action fires when its simulated value now is at least its best within the next 0.5 s. That one generic
rule is what times the smash near the apex over the centroid.

### 5.5 Curiosity and experiments

- **Learning progress (LP)** per region of the link store: the drop in mean prediction error between the last 20
  predictions and the 20 before (IAC). Unpredictable-but-not-improving sources, such as random petting, have LP ≈ 0.
- **Chance of an experiment per decision** = `min(cap, k × curiosity_drive × LP_here + novelty)`. Novelty for a
  never-seen context can lift it to the cap, never above. The cap comes from the Pack: 0.35 for a hatchling, 0.10 for an adult (IAC used 0.35 and 0.15).
  It decays towards 0.01 as LP falls.
- **What can be picked:** options scoring ≥ 50 % of the best, never tagged `no_explore`, never during focus, at most 1
  per 5 min on the desktop, 1 per 20 s while the owner is playing (they threw the Pet or a prop in the last 30 s),
  and 1 per 10 s in an Encounter. Weighted by expected LP, so untried pairs come first.
  Always shown with ❓ and the why: *"trying shoot on the ball: no idea what happens"*.
- **Two draws per decision, always**, from a seeded PCG generator, so the draw sequence never depends on how many
  options there were.

### 5.6 Amendments to existing designs (each needs the owner's yes)

1. **Air choices:** a Form may name a utility node per Situation, such as `during.falling = "air"`, holding only `air`
   Actions. This needs an ADR amending decision 2026-11-03. Air Actions read `since_event(thrown)`, so no new Situation
   is needed.
2. **Throw appraisal on landing:** `thrown` (−1) is softened by two generic Pack columns:
   `if_air_action = 0.0`, and `if_expected = +0.2` when the throw was a predicted step of the plan the Pet was
   following. The emitter knows both by the time the Pet lands, so practice doesn't make it wary.
3. **Strikeable props:** an optional `HurtBox` with team `PROP` and no Health, so the ball can be hit.
4. **Randomness:** pillar 1 is reworded as in §7. pet-memory rule 2, mind-architecture's "nothing random" and
   neural-mind's exploration argument then need the same change.

### 5.7 Pack data, still data only

```toml
[actions.smash]
tags = ["air", "attack"]
effects = [ { dive = { speed = 400 } },
            { on_contact = { strike = { radius = 120, damage = { per_impact_speed = 0.02 } }, bounce = 0.0 } } ]
considerations = [ { input = "plan_value", bookends = [0, 0.5], curve = "linear_1_to_2" } ]

[actions.follow_cursor]          # the existing BEH-06 Action, unchanged; nothing about throws is added
tags = ["social", "play"]
limits = { during_focus = "never" }   # any Action may carry limits; the planner respects them

[learning]
link_ratio = 2.0
min_yes = 3
planner = { depth = 3, beam = 4, discount_per_s = 0.1 }
experiments = { cap = 0.35, adult_cap = 0.10, floor = 0.01, band = 0.5, desktop_rate = "1 per 5m", play_rate = "1 per 20s" }
unknown_link = 0.3            # one generic guess for any link never seen, not per combo
fallback_blend = 0.5
link_features = ["over_group", "height", "enemies_present", "owner_holding", "cursor_near", "ball_flying", "release_speed"]
```

A closed Rust vocabulary reads all of it, with no expressions or scripts, so the 2026-11-02 decision holds. New kinds of
effect, feature or simulator need Rust.

### 5.8 Storage, explanations, guards

**Storage:** Progress per Character, in the JSON save ([persistence.md](persistence.md)) under `learned`. Keys are
strings, such as `"[world]|cursor_near=yes,enemies=yes→owner_holding"`. Links whose features vanish from the Pack go dormant. The
`Rng` seed and draw count are saved with them. Typical size is 20–60 KB. **Inference.**

**Every plan explains itself**, link by link, with simulated links marked ⚙: *"Going to your cursor: ⚙ walking gets me there in 2 s;
in a fight you pick me up by your cursor 3 of 4 times and throw me at orcs 3 of 4 times; ⚙ from 570 px a smash lands
at 1,562 px/s and does 31 damage, so it should defeat 5."* The same record tells the separate communication system
the current goal, for example an Emote, without being part of learning.

| Risk | Guard |
|---|---|
| Spurious links ("smash on Tuesdays") | Ratio test against the base rate; only Pack-listed features; refinement needs 10 tries |
| Owner pets constantly | Contingency baseline; LP ≈ 0, so no curiosity hook |
| Chaos from experiments | 50 % band, rate caps, ❓ label, no focus-time experiments, decay with LP |
| Hanging round the cursor | `limits` and `since_last` on any Action; no pick-up lowers only that link |
| Stuck | Experiments revisit regions where LP returns, for example after a Pack or Form change |
| Old lessons | α floor 0.25: four unrewarded tries take 0.6 to about 0.25 |

### 5.9 Cost at 30 Hz

| Work | When | Cost (**Inference**) |
|---|---|---|
| Feature cache, about 10 features | every tick | ~0.5 µs; centroid over 1,000 enemies ~1.5 µs |
| One simulated node (arc, about 30 steps, plus an AoE count) | per planner node | ~1–2 µs |
| Plan: 10 + 40 + 40 nodes | on events, at most 1/s | 0.1–0.2 ms; it can run over 2–4 ticks, keeping the worst tick under 0.05 ms |
| `air` replan, depth 1, while falling | every tick in the air | ~10–20 µs |
| Credit and link updates | per reward or result event | < 2 µs |

Companion average is about 5 µs a tick, 0.1 % of the ~5.5 ms headroom. Depth 4 (about 400 nodes, ~0.8 ms) fits only
the Encounter budget. RAM: about 500 links × 40 B + cells + residuals, 20–40 KB.

## 6. Scenario A, learning the combo, event by event

Building blocks: the owner's throw, `follow_cursor` (existing) and `smash` (given). Nothing else is added. Orcs have
28 health. g = 2,000 px/s². Smash: `dive` at 400 px/s, then damage 0.02 × impact speed.

| # | Event | What the Mind does | What is learned |
|---|---|---|---|
| 0 | Practice session on the desktop | The Pet follows the cursor as usual (play), and the owner grabs it 3 times while it is near | World link [cursor_near, play] → owner_holding: 3 of 8 near-cursor moments, 0.4, against ~0 when far |
| 1 | Practice 1: the owner flicks it up with the grab string, apex 520 px | `air` node: smash (untried) or keep falling. The draw 0.61 > 0.35 means no experiment, so it keeps falling. | World link [held] → thrown, apex ~520 (1/1) |
| 2 | Practice 2: apex 610 | Experiment ❓ draws `smash`. Impact ⚙ √(400² + 2·2000·610) ≈ 1,612 px/s, measured 1,600 | Residual: landing +45 px ahead. The owner pets 1.2 s later: cell `smash \| airborne_high` +0.55 |
| 3 | Practice 3: apex 580 | Plan value of smash is now positive, so it smashes on purpose | Apex mean 570 (n 3); residual +40 px (n 2) |
| 4 | First Encounter: 5 orcs within 120 px; Pet 400 px from them, cursor 200 px away | Bite: walk 4 s, 12 damage, no kill, 0.05 × 0.66 = 0.03. Chain: ⚙ `follow_cursor` 2 s → [cursor_near] → held, blended 0.5 × 0.4 + 0.5 × 0.3 = 0.35 → [held] → thrown, 0.5 × 0.8 + 0.5 × 0.3 = 0.55 → towards the group (guess 0.3) → ⚙ smash from 570 px: 1,562 px/s, 31 damage, 5 defeated = 1.25; × 0.59 discount (5 s) = **0.04**. | `follow_cursor` wins, narrowly, on value. "Why" marks the guesses. |
| 5 | It comes to the cursor; the owner grabs it and flicks it at the orcs | Emitter: `thrown` was a predicted plan step, so `if_expected` gives +0.2 | Encounter links near → held, held → thrown and "towards the group": each 1/1 → 0.67 |
| 6 | Falling past apex at dx 300, 280 … | `air` replans every tick: ⚙ + residual (40 px) say hits peak at dx = 40 | |
| 7 | Smash at dx 40, lands on the midpoint; 5 defeated | +1.25 | Chain value 0.67 × 0.67 × 0.67 × 1.25 × 0.59 = **0.22**, no guesses left |
| 8 | After 5 good runs | Saved as a routine `follow_cursor → [held] → [thrown] → smash` | "Why" as in §5.8 |

**How fast:** if practice taught the near-cursor link (row 0), the Pet goes to the cursor in its first Encounter, on value
alone. That value rests on the generic guess (0.3), and the margin is thin. With a guess of 0 it would need an experiment. The chance per Encounter is then
1 − (1 − p_explore × p_pick)³ over about 3 eligible decisions. For a hatchling (0.35 × ⅓) that is about 0.31, so about
3 Encounters. For an adult (0.10 × ⅓) it is about 0.10, so about 10. The guess is a design lever, open decision 6. If the owner never grabbed it near the cursor during play, the chain
can't be planned until they do. Grabs always happen with the cursor on the Pet, so any 3–5 grabs teach it.
"Right after the first practice" needs that link and a guess above about 0.25. **Inference.**

## 7. Randomness: three options

| | A. Fixed rules (untried bonus in list order) | B. Seeded, bounded, labelled experiments | C. Free randomness |
|---|---|---|---|
| Explainable | Every choice | Every choice; experiments say "❓ trying X" | "Why did it do that?" has no answer |
| Testable | Exact | Exact given seed and draw count; property tests over 1,000 seeds check caps and bands | Flaky |
| Feels | Methodical, a visible sweep through options | Curious and young, like IAC's developmental stages; Petz's "constrained randomness" shipped | Erratic: NERO's authors and Trico's critics (neural-mind.md; §2) |
| Exploration decays | Only by count | With learning progress, so it returns when the world changes | Never |
| Escapes ties and loops | Poorly: same order every time | Yes | Yes |

**Recommendation: B.** Keep A's untried bonus only as the fallback if B is refused. Proposed wording for pillar 1:
> **Alive** — it behaves for reasons, and its reasons are visible. When it experiments it shows it (❓), its experiments
> stay within limits and fade as it learns, and nothing it does goes unexplained.

## 8. Scenario B, teaching by reward, event by event

`shoot` fires a Projectile at the attended target. The ball has a Hurt box (amendment 3). Base scores with the ball
flying: watch 0.30, chase 0.25, pounce 0.20, shoot→ball 0.25. The band is ≥ 0.15. Experiment chance in this novel
context is the hatchling cap, 0.35. Throws come about 30 s apart, within the play rate of 1 per 20 s. Weights are
untried 1 and tried 0.3, and the seed is shown in the dev panel.

| Throw | Draws | What happens | Learned |
|---|---|---|---|
| 1 | 0.31 < 0.35 → experiment; pick among chase, pounce, shoot | ❓ chase | chase × ball: no reward |
| 2 | 0.72 → no experiment | watches (0.30) | |
| 3 | 0.12 → experiment; pounce 1, shoot 1, chase 0.3 | ❓ pounce | no reward |
| 4 | 0.20 → experiment; shoot 1 vs 0.3 + 0.3 | ❓ shoot; ⚙ intercept p 0.8; strikes at 0.8 s; marker at 1.0 s; petting at 2.5 s | Link `shoot → prop_struck` 1/1; cell `prop_struck \| owner_threw_ball` 0.6 − 0.05 = +0.55 |
| 5 | 0.88 → no experiment | plan_value 0.8 × 0.55 × confidence ½ = 0.22 → ×1.44; shoot **0.36** beats watch 0.30. One reward changed it. | |
| 6–7 | Rewarded twice more | Confidence ¾, value 0.33 → ×1.66 → 0.41 | 💡 when the cell reaches n = 3 |
| 8+ | | Shoots by default: *"you petted me after 3 of 3 hits on a flying ball"* | |

**Realistic count:** each throw gives a shot with chance of about 0.35 × ⅓ to 0.35 × 0.6, so about 5–8 throws for a
hatchling (seed-dependent) and 15–25 for an adult (cap 0.10). With an owner command (Track 3) it takes 1, then 1 reward before it shoots by default. A resting ball is a different context and learns separately.
If the owner pets constantly, the baseline rises and the lesson fades.

## 9. Phasing

**MVP (item 4) must reserve:**
1. A `DecisionRecord` ring of about 64: the credit trail.
2. One feature vocabulary for considerations and learning, with every read returning a value and its cause.
3. Actions defined by Effects on shared features. No outcome-named special cases.
4. Physics and ability rules as pure `pet-core` functions callable for one Action ahead.
5. `during.<situation>` in the Form format.
6. Appraisal variants decided on landing; `HurtBox` not tied to Health.
7. A `learned` section and the `Rng` seed and counter in Progress, from save version 1.
8. All randomness through one Mind-owned seeded generator, two draws per decision, or none at all if option A is chosen.

**MVP taste:** outcome cells for `sit`, `look_at_cursor`, `chase_cursor`, `roll`, by `time_of_day` (4) × `cursor_near`
(2). Reward is `petted` through the credit window, with ❓ experiments capped at 1 per 5 min. Learning shows as a
factor ×1.0–×1.3, a 💡 Emote, and the tooltip "why". About 500 lines of `pet-core` with Given-When-Then tests.
**Inference.** First test: *Given* seed 7 and a fresh Pet, *when* the owner pets it 1 s after each of 3 evening sits by
the cursor, *then* sit's factor in {evening, near} is > ×1.1, its cause lists 3 episodes, and replaying with seed 7
gives the same choices.

| Track | Adds |
|---|---|
| **1 Behaviour** | Links with ratio test and refinement, learning-progress curiosity, two speeds, dev-panel link browser with forget, routines |
| **2 Interactions** | Ball prop with Hurt box, treats, middle-click marker, `shoot`-style toys (Scenario B), residuals, cue learning: a Gesture just before a rewarded Action becomes a context feature |
| **3 Encounters** | Group features, `simulate()` for abilities, the `air` node, the depth-3 planner, owner commands as demonstrations (Scenario A) |

## 10. Open decisions for the owner

1. **Randomness (§7):** accept B and the pillar wording? If so, pet-memory rule 2, mind-architecture's and
   neural-mind's "nothing random", and this doc's MVP item 8 change with it.
2. **Air choices (amendment 1):** an ADR amending 2026-11-03.
3. **Marker Gesture:** I'd pick a middle click on the Pet. A plain click clashes with poke combos (INT-04). A
   `RegisterHotKey` hotkey isn't a hook but breaks §10.2's mouse-only rule. A clickable "good?" Emote needs REN-04
   changed.
4. **Throw appraisal (amendment 2):** are the `if_air_action` and `if_expected` values right?
5. **Experiment caps:** 0.35 for a hatchling and 0.10 for an adult, 1 per 5 min on the desktop. Is that too lively or
   too tame?
6. **Simulator fidelity and the unknown-link guess:** should the planner simulate Materials and the grab string's spring,
   or learn them as residuals? And how bold should the guess for never-seen links be? 0.3 asks early; 0 waits for experiments.
7. **How deep may plans go?** 3 everywhere, or 4 in Encounters only?
8. **Knowledge across Forms:** keys are Action ids, so shared Actions keep their links. Should a new Form start fresh
   instead?
9. **Can the owner see and erase what it learned?** I'd show a "what I've learned" list, link by link.
10. **Proposed glossary additions:**
    - *Feature:* a named, binned reading of the world.
    - *Link:* a learned "after this Action, in this context, that tends to follow".
    - *Experiment:* a seeded, labelled exploratory choice.
    - *Routine:* a discovered multi-step plan. Not pet-memory.md's per-Action "Habits" counters.
    - *Marker:* the Gesture that says "that one".
    - *Prop:* a Mindless, throwable thing, such as the ball.

## Sources not linked above

- Sutton 1991, "Dyna, an integrated architecture for learning, planning, and reacting", *SIGART Bulletin* 2(4).
  **Summary.**
- Eurogamer's Black & White retrospective was read through a [reader mirror](https://hn.jakechampion.name/reader/https://www.eurogamer.net/black-and-white-lionhead-peter-moylneux-retrospective-google-deepmind-gemini-ai). **Summary.**
- Grand, Cliff & Malhotra 1996 (Creatures) and Rescorla-Wagner are marked in [pet-memory.md](pet-memory.md) §1.
