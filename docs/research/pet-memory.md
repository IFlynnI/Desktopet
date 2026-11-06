# Pet memory: what to learn from brains and games, and three shapes for it

**Researched:** 2026-11-06, for roadmap item 4, the behaviour engine (BEH-01 to BEH-05, §8.1).
**Purpose:** propose how the Pet's memory works, so that any new kind of event can feed it and what it remembers turns
into behaviour you can see and explain. This doc ends with a recommendation and the decisions only the owner can make.

**How to read the marks.**

| Mark | Meaning |
|---|---|
| **Read** | I read the primary text for this doc. |
| **Summary** | I found the primary source, but read only its abstract, its listing, or write-ups about it. Take the lesson, not the details. |
| **Inference** | My reasoning or estimate. Not measured. |

## Bottom line

- **Cost won't decide this.** All three candidates cost microseconds per tick and a few to a few hundred kilobytes. The
  whole app gets 1 % of the machine (§6.2), and 0.46 % is about 15 % of one core. The spike already uses 0.46 %, so
  about 0.54 % is left, roughly 17 % of a core or about 5.5 ms per 33 ms tick. The recommended memory uses about 0.05 %
  of that headroom, and even the heaviest candidate stays under 3 %. **Inference.**
  As in [simulation-architecture.md](simulation-architecture.md), they differ on other things: how well you can
  explain them, how much a designer can tune, whether the Pet learns, and how ready they are for the AI track.
- **Recommendation: candidate B, "episodes and associations".** Events go into a small short-term buffer. Each one nudges
  a mood and an opinion about its subject, such as the cursor, the owner or orcs. Opinions settle into long-term memory
  when the Pet sleeps. Designers write it as a table per event kind in the Pack, as plain as RimWorld's thoughts.
- **Memory only offers numbers to the scoring, it never decides.** It adds inputs, such as "how do I feel about the
  cursor" or "minutes since I last did this", that any consideration can read. That works with either utility shape in
  [simulation-architecture.md §5](simulation-architecture.md#5-utility-ai-prior-art).
- **Every number keeps its reason.** Each answer the memory gives comes with the episode or opinion that caused it, so
  the Pet can show "wary: thrown 3 times, the last 4 minutes ago".

## Rules from the PRD the memory must obey

1. **No guilt (§4, "never").** The owner being away or idle is never stored as something bad. Mood drifts back to a
   baseline set by personality traits, and time alone never takes it below that baseline. The owner-presence kinds
   (`owner_idle`, `owner_away`, `owner_back`) are emitted by the core itself, so they are a short fixed list in
   `pet-core`. `pet-pack validate` rejects a negative valence on them, and the core clamps it to ≥ 0 anyway.
2. **Nothing random (pillar 1).** Variety comes from memory instead: familiarity wears off, and "I did this a minute
   ago" counts against doing it again. That rules out Dill's weighted-random pick. Ties break in a fixed order.
3. **Privacy (SEN-01/02).** The memory stores only what sensing is allowed to see: cursor, idle time, the Pet's own
   interactions, the time of day and game events. It never stores app names or titles. Opt-in AI-track context stays
   out of it unless that track decides otherwise.
4. **Memory is part of Progress (GLOSSARY.md), kept per Character** and saved in SAV-01's versioned schema, so it is
   covered by its backups and export.
5. **`pet-core` has no IO.** Time comes in as a tick count. Decay is worked out **lazily**: each value stores when it
   was last updated, and a read computes how much has faded. Nothing runs every tick, so there's nothing to poll, and
   Given-When-Then tests can jump time forward.
6. **Minds are shared.** A boss can use the Pet's Mind, so memory belongs to an Actor, not to "the Pet". Basic enemies
   have none.

## The picture

```text
 sensors / Situations / Encounter rules          (Rust: one emitter per new source)
        │  MemoryEvent { kind, subject, intensity, at }
        ▼
 ┌─ Appraisal ── Pack table: kind → valence, arousal, salience, learn rate, cues
 │      ▼
 │  Episode ──► short-term ring buffer (last 64)          "what just happened"
 │      ├─────► Mood (valence, arousal), leaks to baseline  "how I feel"
 │      ├─────► Association[subject]: opinion + familiarity "what I think of X"
 │      └─────► Recent counters per kind and per own Action "how often lately"
 │                                  ▲
 │   consolidation (on Sleep) ──────┘ fast opinion → slow opinion; big episodes → landmarks
 ▼
 Memory queries ──► consideration inputs ──► utility score ──► Action + Emote + "why"
```

**If you're used to web apps, this is event sourcing.** Episodes are the append-only event log. Mood, associations and
counters are **projections**, the read models updated as each event arrives. Consolidation is **compaction**: the old
log is folded into a snapshot and dropped. Considerations are queries against the read models. Lazy decay is a computed
column, worked out from a stored timestamp when you read it. The difference from a web app: the log is tiny and
deliberately forgetful.

## 1. What each source teaches

| Source | What matters for the Pet | Mark |
|---|---|---|
| Working memory: [Baddeley 2000](https://pubmed.ncbi.nlm.nih.gov/11058819/), the episodic buffer | "Now" is a small, bounded store that binds what is happening into episodes. That means a fixed-size short-term buffer, not an unbounded log. | Summary |
| Episodic vs semantic: [Tulving 2002](https://pubmed.ncbi.nlm.nih.gov/11752477/) | Episodes are "what happened when"; semantic memory is "what is true". The Pet needs both: "thrown at 14:02" and "the cursor is rough". | Summary |
| Consolidation: [McClelland, McNaughton & O'Reilly 1995](https://stanford.edu/~jlmcc/papers/McCMcNaughtonOReilly95.pdf) | A fast store learns at once, and a slow store learns by replay, so one event can't wipe old knowledge. That gives two rates: a fast opinion and a slow one. | Summary |
| Sleep: [Diekelmann & Born 2010](https://doi.org/10.1038/nrn2762) | Sleep is when new memories are consolidated. So the Pet's own Sleep Action is the natural moment to consolidate, which is also visible. | Summary |
| Salience: [McGaugh 2004](https://pubmed.ncbi.nlm.nih.gov/15217324/), the amygdala | Emotionally arousing events are kept longer and more strongly. Each event kind has a **salience** that sets how long it lasts and whether it becomes a landmark. | Summary |
| Forgetting: [Murre & Dros 2015](https://doi.org/10.1371/journal.pone.0120644), replicating Ebbinghaus; [Anderson & Schooler 1991](https://dyurovsky.github.io/learning-humans-machines/class/13-class/papers/anderson1991.pdf) | Forgetting is fast at first, then slow, roughly a power law rather than one exponential. Anderson & Schooler tie memory strength to recency and frequency. Two exponentials, fast and slow, come close and are easier to tune with half-lives. | Summary |
| Habituation: [Rankin et al. 2009](https://pmc.ncbi.nlm.nih.gov/articles/PMC2754195/) | A harmless stimulus that keeps repeating gets less response, and the response recovers after a pause. Sensitisation is the opposite: a strong aversive stimulus raises the response to others. That gives a **familiarity** value with a recovery time, and arousal that builds up across rapid pokes (INT-04). | Summary |
| Conditioning: [Rescorla & Wagner 1972](https://about.illinoisstate.edu/vfdouga/files/2021/01/RescorlaWagner.1972.pdf) | Learning follows surprise: `V += α·(λ − V)`. When the outcome λ matches what the Pet expected (V), it learns nothing. This is one line of code with a tunable rate, and it explains extinction too: petting slowly undoes wariness. | Summary |
| Affect: [Russell 1980](https://doi.org/10.1037/h0077714), the circumplex model | Mood fits on two axes, **valence** (pleasant to unpleasant) and **arousal** (calm to stirred). Two floats are enough. | Summary |
| Creatures: [Grand, Cliff & Malhotra 1996](https://www.sussex.ac.uk/informatics/cogslib/reports/csrp/csrp434.pdf) | Each synapse has a **short-term weight** that relaxes back towards a **long-term weight**, which moves slowly, in effect a moving average of the short-term shocks. One bad experience means strong avoidance now, while the long-term view stays milder. After an action, a decaying **susceptibility** lets a delayed reward reach it. Stroking rewards and slapping punishes. | **Read** |
| The Sims: [Forbus & Wright 2001](https://qrg.northwestern.edu/papers/Files/Programming_Objects_in_The_Sims.pdf); Sims 2 [designer diary](https://www.gamespot.com/articles/the-sims-2-designer-diary-3/1100-6075605/) | Objects advertise what they satisfy. In Sims 2, memories carry a sign, recent ones weigh more, they shift mood and over time personality. The Sims' needs decay and punish, which §4 rules out. | Summary |
| Black & White: [Molyneux's postmortem](https://gamedeveloper.com/design/postmortem-lionhead-studios-i-black-white-i-) and [Wikipedia](https://en.wikipedia.org/wiki/Black_%26_White_(video_game)) | The creature learned "like a clever puppy" from slaps and strokes. Desires were perceptrons and opinions decision trees, on a belief-desire-intention model. Lesson: an **opinion about a subject**, learned from your feedback, is what players read as personality. | Summary |
| RimWorld: [Mood](https://rimworldwiki.com/wiki/Mood), [memory thoughts](https://rimworldwiki.com/wiki/Thoughts/Memory_Misc) | Each thought is data: a mood offset, a duration in days, a stack limit, and a stacking multiplier, often 0.75. Mood moves to its target at most +12 or −8 per in-game hour. The mood tooltip lists every thought, which is the explainability model. | Summary (wiki) |
| Dwarf Fortress: [2018 coverage](https://gamedeveloper.com/game-platforms/-i-dwarf-fortress-i-is-giving-its-dwarves-the-ability-to-form-and-dwell-on-memories), [wiki](https://dwarffortresswiki.org/index.php/Memory_(thought)) | 8 short-term slots keep the strongest emotion+event memories. Once a year one may replace a weaker memory in 8 long-term slots, and dwarves "dwell" on them again. Fixed slots and strongest-wins is a cheap, bounded consolidation rule. | Summary |
| Shadow of Mordor: [GDC 2018 talk](https://gdcvault.com/play/1025150/), [patent report](https://gamespot.com/articles/warner-bros-now-has-a-patent-for-shadow-of-mordors-nemesis-system/1100-6487210/) | Enemies remember what happened between you and them, and that is what makes them memorable. Warner Bros holds a US patent on the Nemesis system. Remembering individual enemies is generic, but its hierarchy and promotion mechanics shouldn't be copied without checking. This is not legal advice. | Summary |
| F.E.A.R.: [Orkin 2005](https://www.cs.uky.edu/~sgware/reading/papers/orkin2005agent.pdf) | Sensors write every kind of knowledge into one **common record format**, a `WorkingMemoryFact`, and each attribute has a 0–1 confidence. The planner queries it through a few generic calls: find a matching Fact, count them, or take the one with the highest confidence. Cleanup was ad hoc, and Orkin suggests giving every Fact an expiration time. | **Read** |
| Kismet: [MIT Kismet pages](https://groups.csail.mit.edu/lbr/sociable/emotions.html); [Breazeal 2003](https://doi.org/10.1016/S1071-5819(03)00018-1) | Each perceived trigger is tagged with arousal, valence and stance. Emotions compete winner-take-all and drive both the behaviour and the face. That is appraisal as a tag on the event, and an expressed emotion as the Emote. | Summary |
| Generative Agents: [Park et al. 2023](https://arxiv.org/html/2304.03442v2) | A memory stream scored by recency (decays ×0.995 per game hour), importance (1–10, set when stored) and relevance (embedding similarity). Each is scaled to 0–1 and summed with weights of 1. Recency counts from when a memory was last *retrieved*, so recall works like rehearsal. Reflection runs when the importance of recent events passes 150, about 2–3 times a game day. We can copy recency, importance and reflection without an LLM. Relevance needs embeddings, so we'd swap in tag overlap. | **Read** (numbers) |

## 2. Three candidates

All three share the same event going in. Only the Rust code that *emits* events knows about sensors or combat:

```rust
struct MemoryEvent {
    kind: KindId,               // Pack string id, e.g. "thrown", "petted", "encounter_won"
    subjects: [Option<SubjectId>; 2], // up to two: ["cursor", "owner"], ["orc#17", "orc"]; none for "night fell"
    intensity: f32,             // 0..1, e.g. throw speed or how long the petting lasted
    at: Tick,                   // simulation tick, 30 per second
}
```

**Two subjects** let a desktop interaction teach about the hand and the person separately. The Pack gives each subject
its own learn rate, so the Pet can be wary of the cursor and still fond of you. Encounters use the same slots for an
individual and its kind, so `orc#17` and `orc` both learn.

Hit by an orc three times in one second? Same-kind, same-subjects events within a short window **merge into one
episode** with a count, the way RimWorld stacks thoughts. That keeps an Encounter from flooding the buffer. Mood,
opinion and arousal still update once per event, so the poke-combo build-up is kept. Only the buffer row is shared.

### A. Thoughts, RimWorld-style

**Shape.** Each Pack row is a thought: the event kind, a mood offset, a duration, a stack limit, a stack multiplier,
whether it fades linearly, and which tags it affects. Memory is a list of active thoughts, capped at about 32.

**Decay, consolidation, retrieval.** A thought simply expires. There is no consolidation and no long-term memory.
Retrieval is "the sum of active thoughts with tag T".

**Feeding the scoring.** Inputs are `mood` and `thought_sum(tag)`. For example, while "Thrown" is active, "follow
cursor" scores lower.

**Strength.** Simplest to build, tune and explain. **Weakness.** It doesn't learn: every throw causes the same 10 minutes
of wariness, whether it's the first or the fiftieth. It covers BEH-04's "short-term memory", but not "grows with you".

### B. Episodes and associations, two speeds (the recommendation)

```text
Episode      { kind, subjects, valence, arousal, salience, count, at }   ~32 B, ring buffer of 64
Mood         { valence, arousal, at }                                   leaks to the trait baseline
Association  { subject, opinion_fast, opinion_slow, familiarity,        ~40 B, cap ~256 by least-used
               encounters, last_at, strongest_episode }
KindStats    { kind, last_at, recent_count }                            also one per own Action
Landmark     { episode, why }                                           strongest 16, kept for good
Rhythm       { owner_active[24] }                                       per-hour average, 96 B
```

**Appraisal.** The Pack turns `kind` into valence, arousal and salience, scaled by `intensity`. It's the same table as
A's, with a few more columns.

**Mood.** A push of `valence × salience` is added, then fades back to the baseline with a half-life. That works like
Kismet's affect and RimWorld's target, with an optional rate limit like RimWorld's +12/−8 per hour.

**Opinion: conditioning.** Each subject's fast opinion learns Rescorla-Wagner style: `fast += α·(λ − fast)`, where the
outcome λ is `valence × intensity` and α is that subject's learn rate from the Pack. Cues named by the Pack that were
seen in the last few seconds also learn. This is Creatures' susceptibility: `cursor_near` before `thrown` teaches "a
cursor coming close means trouble". Example with valence −1, full intensity and α = 0.3: three quick throws take the
cursor's opinion from 0 to −0.30, −0.51, −0.66. Thrown 30 s apart, relaxation trims that to about −0.65. One petting at
λ = +0.6 then brings it back to about −0.28.

**Two speeds: consolidation.** Between consolidations, the fast opinion relaxes back towards the slow one, with a
half-life of about 20 minutes. Whenever the Pet sleeps (with a fallback every few hours awake), the slow opinion moves
part of the way towards the fast one, `slow += k·(fast − slow)`, with k ≈ 0.1–0.3. Episodes with salience over a
threshold become landmarks, and the weakest landmark makes way, as in Dwarf Fortress. Everything else in the buffer is
just overwritten. The result: one bad throw makes the Pet jumpy for minutes, but its view of you changes only a little
after sleeping on it. That is what Creatures' short- and long-term weights do, and what McClelland's fast and slow
stores do.

**Habituation and sensitisation.** On each episode, `familiarity += h·(1 − familiarity)`, recovering towards 0 with a
recovery half-life. A cursor that keeps drifting by stops getting looked at, and is interesting again after a break.
Aversive episodes raise arousal, which lowers the threshold for "annoyed" and drives the poke combo escalation (INT-04)
without a special counter.

**Habits.** KindStats per own Action gives "ticks since I last did X" and "times in the last 10 minutes". One input
gives variety without dice and damps the oscillation Lewis describes. The rhythm learns when the owner is usually
active, from idle time only (SEN-01), so the Pet can perk up just before you usually come back. It only anticipates;
being away is never held against you.

**Feeding the scoring.** These inputs are all normalised to fit Infinite Axis bookends or Dill's addend and multiplier:
`mood.valence`, `mood.arousal`, `opinion(subject)`, `familiarity(subject)`, `since_last(kind|action)`,
`recent_count(kind, window)`, `rhythm.owner_usually_active`.

### C. Memory stream with retrieval and reflection, Generative-Agents-style without an LLM

**Shape.** Keep every episode for about a day, up to about 2,000, each with an importance from the Pack.

**Retrieval.** Score each episode as `w_r·recency + w_i·importance + w_t·tag_overlap(episode, query)` and take the top
k. Each Action names the tags its query uses.

**Reflection.** When the summed importance passes a threshold, Pack rules turn episode patterns into beliefs. For
example, "3 or more `thrown` by cursor in a day" becomes the belief `cursor_is_rough` with strength 0.7. Beliefs decay
unless they're confirmed again.

**Feeding the scoring.** `retrieved_valence(query)` and `belief(name)`.

**Strength.** It makes the richest stories, and the episodes are ready to become text for the AI companion. **Weakness.**
Three weights interact, so a designer can't predict what gets retrieved. Reflection rules are a small rule language,
which comes close to the "Packs are data only, no scripting" decision (2026-11-02).

## 3. How they compare

| | A. Thoughts | B. Episodes + associations | C. Stream + reflection |
|---|---|---|---|
| **New event kind** | One Pack row, plus Rust to emit it if no emitter exists yet. No change to memory code or save schema. | Same | Same, plus any reflection rules that use it |
| **New subject (orc, orc#17)** | Not applicable | Automatic: a new association. An event may name an individual and its kind, so both learn, as in Creatures' generalisation. | Automatic |
| **What is saved** | Active thoughts with their remaining time | Mood, associations (fast and slow), familiarity, KindStats, landmarks, rhythm. The episode buffer is optional. | All episodes and beliefs |
| **Save size** | < 2 KB | ~10–25 KB as text | ~100–300 KB as text |
| **RAM per Actor** | ~1 KB | ~15 KB. For an Encounter: the Pet and one boss, so ~30 KB. Basic enemies: 0. | ~100 KB |
| **Per-tick CPU** | ~0.5 µs | ~1–3 µs: about 20 memory reads per decision, each a hash lookup and one `exp`. Work happens only on events and reads. | ~50–150 µs if 20 queries each scan 2,000 episodes. A per-tick cache cuts that. |
| **Share of the ~5.5 ms/tick headroom** | ~0.01 % | ~0.05 % | ~1–3 % |
| **Tuning as Pack data** | Offset, duration, stack limit and multiplier per kind | Valence, arousal, salience, α, cues per kind; half-lives, k and caps once per Pack; traits set the baseline mood | Importance per kind, 3 retrieval weights, reflection rules |
| **Explainable** | Very: list the active thoughts | Very: each read returns its value and cause, such as `opinion(cursor) = −0.66 ← thrown ×3, last 4 min ago` | Partly: "these 5 episodes scored highest" is hard to read |
| **Learns and grows** | No | Yes, at two speeds, with familiarity and habits | Yes, the most |
| **Ready for the AI track** | Low | Good: landmarks and recent episodes are short, structured records | Best |

All costs and sizes are **Inference**: they're estimates from record sizes, not measurements. Shown as an Action's
"why", B's explanation would read something like: *Emote ❗, Action backs away. Top factor: opinion(cursor) −0.66,
from 3 throws, the last 4 minutes ago.* The Mind keeps the top factor and its cause with each decision, and the same
record feeds the Emote, a "why" line in the hover tooltip, and the dev balancing panel.

## 4. Recommendation

**Build B, presented to designers like A, and storing episodes so C stays possible.**

- **Why not A:** it can't learn. A thrown Pet would be just as wary the fiftieth time as the first, which doesn't match
  "grows with you" or "intelligent-looking".
- **Why not C:** it isn't too slow. It's too hard to tune and predict, and reflection rules start to look like
  scripting. Its one real advantage, rich episodes for the AI track, B can get later by making the buffer bigger and
  adding retrieval behind the same query interface.
- **Why B:** each mechanism is one line of maths with a half-life or rate a designer can read. The two speeds give
  "jumpy now, forgiving later" for free, and every number carries its cause.

**What the Pack holds (TOML, like the rest of the Pack):**

```toml
[memory]                      # once per Pack; traits set the mood baseline
episodes = 64
landmarks = 16
opinion_fast_half_life = "20m"
consolidation_rate = 0.2      # k: how far slow opinions move towards fast ones at each sleep
familiarity_recovery = "2h"

[memory.events.thrown]
valence = -1.0                # -1..1, how bad or good it is; the outcome is valence x intensity
arousal = 0.9                 # 0..1, how stirring
salience = 0.7                # 0..1, how memorable; >= landmark threshold means kept
learn_rate = { cursor = 0.3, owner = 0.05 }   # alpha per subject: wary of the hand, barely of you
mood_half_life = "10m"
cues = ["cursor_near"]        # cues seen just before it also learn
cue_window = "3s"
```

A consideration reads it like any other input, for example
`input = { memory = "opinion", subject = "cursor" }`, with bookends `[-1, 1]` and a curve.

**Encounters later.** `encounter_won`, `hurt_by` (subject `orc`), `healed_by` (subject `owner`) and `fought` (subjects
`orc#17` and `orc`) are just new Pack rows plus emitters. "The Pet hesitates before orcs after losing", or "fights
harder beside you after you heal it", fall out of `opinion` with no new memory code. The §10.2 bond meter could be
`opinion(owner)`. That's a hypothesis to prototype, not a decision.

**First tests (Given-When-Then, in `pet-core`):**

- *Given* a fresh memory, *when* thrown three times at full intensity, 30 s apart, *then* `opinion(cursor)` < −0.6,
  `opinion(owner)` > −0.2, and the cause names `thrown ×3`.
- *Given* that, *when* 40 minutes pass with no sleep, *then* the fast opinion has relaxed most of the way back towards
  the slow one.
- *Given* the owner idle for 8 hours, *then* mood is at the baseline, not below it.

## 5. Decisions for the owner

The owner answered 1 and 3–7 on 2026-11-06, and 2 is reshaped. See [§6](#6-owners-answers-and-follow-ups-2026-11-06).

1. **Which clock does forgetting use?** Hidden freezes the world (decision log, 2026-11-01). Options:
   - (a) simulation ticks only, so the Pet forgets nothing while closed or Hidden;
   - (b) wall-clock for long-term values only, so yesterday's throw has faded by the next morning;
   - (c) wall-clock for everything.

   I'd pick (b): short-term stays frozen with the world, and long-term matches real days.
2. **When does consolidation happen?** Only during the Sleep Action, which is visible and biological, or also on a
   timer? The timer covers a Pet that rarely sleeps.
3. **How long should the Pet hold a grudge?** These set the feel: the `consolidation_rate` and the floor on the slow
   opinion. Should the Pet ever end up lastingly wary of the owner, or must the slow `opinion(owner)` never go below 0?
   The second is closer to "no guilt" and "better together".
4. **What happens to memories a reloaded Pack no longer defines?** Keep them dormant (saved, no effect, revived if the
   kind returns), or prune them after N days? Either way, keys are Pack string ids, never numbers, so Pack edits and
   hot reload can't scramble a save.
5. **Is the episode buffer saved?** If yes, the Pet keeps "what just happened" across a relaunch, for a few KB more. If
   no, only consolidated memory survives, which works like a night's sleep.
6. **Rhythm learning.** Learning the owner's daily active hours from idle time is allowed by SEN-01, but it is a
   profile of the owner. Should it be opt-in or shown in Settings?
7. **Is mood shown?** Only through Emotes and Animation choice, or also as words in the tooltip ("a bit wary of the
   cursor")? This ties into OQ-2.
8. **Landmarks for the AI track.** If the AI companion arrives, may it read landmarks ("the day it beat its first orc")?
   That belongs in the privacy section §10.4 promises.

## 6. Owner's answers and follow-ups (2026-11-06)

### 6.1 What the answers change

| # | Answer | What it means in the design |
|---|---|---|
| 1 | Short-term on ticks, long-term on wall-clock | `pet-core` gets two injected clocks, `Tick` and `WallTime`. The short-term parts run on ticks: episodes, mood, fast opinions, familiarity and KindStats. Slow opinions and rhythm run on wall-clock. Only *negative* slow opinions fade towards 0, with a wall-clock half-life of about 30 days. Grudges heal with time, but fondness never fades because the owner was away (rule 1). Landmarks and records never fade. If the wall clock goes backwards, elapsed time counts as 0. The tick count is the Pet's own life clock: it is saved in Progress and resumes on launch, so stored tick timestamps stay valid. |
| 2 | Consolidation not tied to Sleep | Consolidation runs when any Action tagged `rest` has run for at least 30 s ([mind-architecture.md](mind-architecture.md) already tags Actions `rest`). A background timer runs it after 2 h awake with no rest. Make the step depend on elapsed time, `k_eff = 1 − (1 − k)^(Δt / 1 day)`. Δt is *awake simulation time* since the last consolidation, capped at 1 day. Otherwise a week with the app closed would give k_eff ≈ 0.79 on the first rest, and one throw before closing would become a lasting grudge. With that, frequent short rests and one long one move slow opinions by the same total, k = 0.2 per day: eight rests in a day each move 2.75 %, and together they make 20 %. So productivity features can use Sleep freely. |
| 3 | `opinion_slow(owner)` ≥ 0 | A clamp on the slow value only. The fast value may dip, so the Pet can sulk for a few minutes after a throw, but it never keeps a grudge. |
| 4 | Dormant, not pruned | See 6.3. |
| 5 | Episode buffer saved | +2–3 KB in the save. Mood and familiarity are rebuilt on load from stored values and their timestamps, so nothing is replayed. |
| 6 | Rhythm on by default, visible and resettable | Settings shows 24 bars (owner usually active, per hour) and a Reset button. Reset zeroes the 96 bytes. |
| 7 | Mood only through Emotes and Animation; the inspector shows reasons as text | The cause strings already exist. The inspector shows them, and the Pet never shows them as words. |

### 6.2 Salience from how monumental an event is

**What biology and agents suggest.** Surprise grabs attention and memory. In [Itti & Baldi 2009](https://ilab.usc.edu/publications/Itti_Baldi09vr.html),
72 % of gaze shifts went to locations more surprising than average. Surprise is the gap between what you expected and
what came. In conditioning, [Pearce & Hall 1980](https://doi.org/10.1037/0033-295X.87.6.532) make a stimulus *more*
learnable when its outcome was badly predicted. Arousal strengthens consolidation (McGaugh, §1). An experience is
remembered mostly by its peak and its end, not its length ([Kahneman et al. 1993](https://ius.uzh.ch/dam/jcr:5ae9adc9-61ec-4174-b37c-4b752f36c23b/Kahnemann%20et%20al.%20-%20When%20More%20Pain%20is%20Preferred%20to%20Less%20(1993).pdf)).
Generative Agents asks an LLM for importance (§1). We can compute it instead. All of these are **Summary** marks.

**Generic rule.** Any event kind may carry a raw `magnitude`: a throw's speed in px/s, a fall's height in px, enemies
defeated in an Encounter. The Pack says which measure each kind tracks and its unit. Memory keeps
`MeasureStats { n, mean, var, best, best_at }` per (kind, measure), about 24 B. The mean and variance are running
averages over about the last 50 events, so the yardstick follows the owner's current habits. `best` is all-time.
Three signals, each 0..1:

```text
novelty  = 1 if this kind's all-time count is 0, else 0   first time ever, not "first since a break"
surprise = clamp((z − 1) / 2, 0, 1)                     z = (magnitude − mean) / sd; needs n ≥ 5
record   = 1 if magnitude > best (and n ≥ 5), else 0    a personal best, high or low as the Pack says
boost    = max(novelty, surprise, record)
salience = base + (1 − base) × boost × monument_weight   base and weight from the Pack row
```

Novelty here means *first ever*. §2's familiarity is kept per subject and recovers within hours, so it only shapes
attention ("look at the cursor again"). It must not feed the boost, or every throw after a coffee break would count as
monumental.

**What the boost feeds.** All of these are tuned once per Pack, not per kind:

- **Decay.** The half-life of that episode's mood push is multiplied by `2^(m × boost)`. With m = 3, a record's mood
  lasts up to 8× longer: 80 minutes instead of 10. A fast opinion is one number per subject, so it gets no
  per-episode half-life. The larger α below carries the effect, and consolidation moves it into the slow opinion.
- **Learning.** Pearce-Hall style, `α_eff = α × (1 + boost)`, capped at 0.9. A surprising throw teaches twice as much.
- **Landmarks.** An episode with `salience ≥ landmark_at` is a candidate, and the weakest landmark makes way. Records
  go into their own **personal-bests** table instead, one row per (kind, measure), where a new record replaces the old
  one. So a run of records never pushes out other landmarks.
- **Merged episodes** keep their peak magnitude and the last one, the peak-end rule, so a flurry of hits is remembered
  by its hardest.

**Why it stays rare without tuning.** For values that don't trend, the expected number of records after n tries is
1 + 1/2 + … + 1/n ≈ ln n + 0.58. That is about 5 records in 100 throws and 7.5 in 1,000, so "best ever" really is
monumental. If the owner gets better, records come more often, which is a true story: "you're throwing higher lately".
This is standard maths. **Inference** for how it feels.

**Explained as:** *"highest throw ever: 612 px (previous best 540 px, 3 days ago)"*. **Cost:** O(1) per event, plus
24 B per tracked measure. 100 measures take 2.4 KB.

**This extends §2's shapes.** An Episode gains ≤ 4 context features plus peak and last magnitude, about 48 B
instead of 32 B (64 episodes: 3 KB). `MeasureStats` and a personal-bests table are new, about 24 B and 48 B a row.

### 6.3 Dormant memories and what a reload costs

**A memory never points into the Pack.** It holds names only. In RAM, each name is a small number from one **symbol
table** per Character, the way a database stores a foreign key instead of a copy of the row. The meaning of a name,
such as its valence or half-life, is looked up when the entry is *used*, never stored in the entry.

| Entry | Names it holds |
|---|---|
| Episode | 1 kind, ≤ 2 subjects, ≤ 4 context features (time-of-day bin, Form, running Action, Situation) |
| Association | 1 subject |
| KindStats, MeasureStats | 1 kind or Action, plus a measure |
| Landmark, personal best | an episode stub with the same names |
| [teachable-mind](teachable-mind.md) Link | an Action or Situation, ≤ 3 features, a result |

Some names are defined by the core and are always live: `cursor`, `owner`, `owner_idle`. Individuals such as `orc#17`
are made at runtime, not by the Pack, and stay until evicted. Only Pack names can go dormant: event kinds, subject
kinds such as `orc`, Actions and features.

**On a Pack reload,** the loader already builds a name → definition map for the new Pack. Then each symbol in the
table is looked up once, and its slot points to the new definition or to "dormant". No memory entry is touched. A large
Pack has about 300 event kinds, 100 subject kinds, 150 Actions and 50 features. That is 600 lookups at about 30–50 ns
each, so **20–40 µs**. A design without a symbol table would resolve every entry instead: about 1,250 entries × 3
names ≈ 3,750 lookups, still only about 0.1–0.2 ms. Both are well under one tick. Parsing the Pack's TOML and decoding
its images, which a reload does anyway, takes far longer. **Inference**; none of this was measured.

**While dormant,** a query meets the dormant flag and skips the entry, one branch. Decay is lazy, so a dormant entry
costs no CPU. When its name returns, it is read with the *current* definition from its stored timestamp, so a
retuned half-life applies straight away. Dormant entries leave the live caps, so live memories never push them out.
Only removing or renaming Pack names creates dormant entries. A Pack would need hundreds of renames before they reach
even 1,000 entries, about 40 KB, so no cap is needed in normal use. **New question for the owner:** add a safety cap
(for example 5,000 entries, about 200 KB, oldest dormant dropped first), or keep them all forever as answered?
**The save** writes names as plain strings in the JSON ([persistence.md](persistence.md)), so it stays readable by hand.
For about 1,250 entries, the names alone are about 3 × 12 B each, roughly 45 KB. With field names and values,
pretty-printed, it comes to about 100–200 KB. That is still far below persistence.md's 10 MB revisit point.

### 6.4 How memories associate with one another

**Sources.**

| Source | Lesson | Mark |
|---|---|---|
| Hippocampal indexing: Teyler & DiScenna 1986; [Teyler & Rudy 2007](https://doi.org/10.1002/hipo.20350) | The hippocampus stores an *index* that points to cortical patterns, not the memory itself. A partial cue activates the index and brings back the whole episode: pattern completion. | Summary |
| Hebb 1949, *The Organization of Behavior* | Things active together become linked. Co-occurrence isn't cause, which is why [teachable-mind.md](teachable-mind.md) adds a ratio test against the base rate. | Summary |
| ACT-R: [Anderson et al. 2004](https://doi.org/10.1037/0033-295X.111.4.1036) | A memory's activation = its own recency and frequency + activation spread from what is in focus now. Each source spreads less the more things it is linked to: the fan effect. | Summary |
| Soar episodic memory: [Nuxoll & Laird 2012](https://web.eecs.umich.edu/~soar/sitemaker/docs/pubs/1-s2.0-S1389041711000428-main.pdf) | The cue is a partial episode, and the best match scores by how many elements the cue and the memory share, weighted by activation. A "next" command steps to the following episode, so the only explicit link is time order. It predicts an action's outcome by recalling the same action in a similar state, then the next episode. | **Read** |
| [Generative Agents](https://arxiv.org/html/2304.03442v2) | Reflections are stored in the same stream, with pointers to the memories they cite, forming trees. That is an evidence link. | **Read** |
| [HippoRAG](https://arxiv.org/html/2405.14831), NeurIPS 2024 | Indexing theory made into code: a graph of noun phrases links to passages. Retrieval runs Personalized PageRank from the query's nodes, with damping 0.5. Each seed is weighted by specificity, 1 / the number of passages it appears in. That matched or beat iterative retrieval and was 10–30× cheaper. The graph itself is built by an LLM. | **Read** |

**Three ways to link, compared.** All costs and sizes are **Inference**.

| | 1. Shared keys (implicit) | 2. Explicit episode links | 3. Key graph + spreading activation |
|---|---|---|---|
| What links | Two memories that share a name: subject, kind, context feature, Action | Stored edges: next in time, "credited to" (a `thrown` credited to the `cursor_near` before it), "evidence for" (opinion, landmark, Link → episodes) | Weighted edges between *names* that co-occur in episodes (`orc`–`portal`–`evening`), Hebbian, with decay |
| "Reminds me of" | Cue = current names. Score each episode or landmark by shared names, each weighted by specificity (Soar, HippoRAG), plus salience and recency | Follows a chain: what happened next last time | Spread 1–2 hops from the cue names, with fan, and pick the best landmark. It finds things two steps away. |
| Cost per query | 80 episodes and landmarks × ~6 names, about 1 µs. The inverted index (name → episodes) is rebuilt on load, not saved. | One hop per link, < 0.1 µs | 6 cues × fan 20 × 20 = 2,400 edge reads, 5–10 µs. Fine at ≤ 1 per second. |
| Saved | 0 B | Pinned evidence stubs, shared and reference-counted: up to ~1,500 × 32 B ≈ 48 KB (teachable-mind's ~500 Links × 3 refs) | Edges capped at 4,000 × 12 B ≈ 48 KB |
| Risk | None new | Links into the ring buffer go stale when it wraps, so stubs must be pinned | Spurious links, and harder to explain |

**What each would serve.**

- **Retrieval.** A Pack consideration input `reminded(valence)` gives the best match score of a landmark, with its
  sign. If the cursor comes in fast from above in the evening, like the record throw, the Pet tenses ❗. The inspector
  shows: *"reminds me of: highest throw ever, 3 days ago (shared: cursor_near, from_above, evening)"*.
- **The planner.** Teachable-mind's Links remain the only causal arrows the planner follows. Associations help it in
  two ways and never decide. First, two-subject keys let a Link learned on `orc#17` also apply to `orc`. Second, later,
  co-occurrence (approach 3) could suggest which (context, Action → result) pairs are worth testing.
- **Explanations.** Approach 2 gives the chain the inspector prints: decision ← `opinion(cursor)` ← 3 `thrown`
  episodes ← each credited to `cursor_near`. Today `Link.last: [EpisodeRef; 3]` and `strongest_episode` point into a
  64-slot ring that wraps within minutes. So the referenced episode must be **pinned**, copied as a small stub into an
  evidence store, or the "why" goes blank.

**Recommendation.** Associations are a graph only in a loose sense: the MVP needs no graph store.

- **MVP:** approach 1, shared keys with a rebuilt inverted index. That is the hippocampal index and Soar's cue match,
  for about 1 µs and 0 B saved. Add the part of approach 2 that is already implied: time order (free in the ring),
  credit links, and pinned evidence stubs, up to about 48 KB. Teachable-mind's `EpisodeRef` then points at a pinned
stub, an amendment that doc needs. This gives `reminded(...)`, generalisation from individuals to
  kinds, and explanations that survive the ring buffer wrapping.
- **Later, when a consumer exists** (richer "reminds me" behaviour, Link candidates, or the AI companion): approach
  3, about 48 KB and spread only on demand. A full LLM-built graph in the HippoRAG style belongs to the AI track. It
  can be built from the same episodes, names and evidence links without changing them.
