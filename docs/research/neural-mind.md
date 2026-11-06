# A neural network in the Mind? What LLMs, games and the numbers say

**Researched:** 2026-11-06, for roadmap item 4 (the behaviour engine) and Tracks 1 and 6.
**Question from the owner:** would an actual neural network give more intelligent behaviour, since LLMs, today's most
capable software, are built from them?
**Builds on:** [pet-memory.md](pet-memory.md), whose candidate B this doc assumes, and
[simulation-architecture.md §5](simulation-architecture.md#5-utility-ai-prior-art) on utility AI.

| Mark | Meaning |
|---|---|
| **Read** | I read the primary text for this doc. |
| **Summary** | I found the primary source, but read only its abstract, its listing, or write-ups about it. |
| **Measured** | I ran it on this machine (32 threads, Rust 1.97.1, release profile like the workspace's). |
| **Inference** | My reasoning or estimate. |

## Bottom line

- **You are already building the smallest kind of neural network.** In pet-memory.md each opinion is one weight,
  trained by `fast += α·(λ − fast)`. That is the Widrow-Hoff "delta rule", the learning rule of a one-layer linear
  network. Textbook Rescorla-Wagner sums all present cues into one prediction before comparing it with the outcome.
  Doing that would make it exactly such a network, and would add effects like "blocking", where a cue that's already
  trusted stops a new cue from being learned ([Sutton & Barto 1981](https://doi.org/10.1037/0033-295X.88.2.135),
  **Summary**). Black & White's "perceptrons" were this kind too. So the real question is narrower: **should the Mind
  get hidden layers, a deep network?**
- **LLMs are smart because of scale, not because they're networks.** Llama 3 405B learned from 15.6 trillion tokens on
  up to 16,000 GPUs. One Pet gets perhaps tens of clear "good" or "bad" signals a day from one owner. A network trained
  only on that would take years to learn what a designer can write in one Pack row today. **Read** for Llama 3,
  **Inference** for the Pet.
- **Cost won't decide this, again.** A 32-64-16 network in plain Rust takes **1.25 µs** per pass, about 0.02 % of the
  tick headroom, and 12.6 KB of weights. **Measured.** Frameworks add about 0.8 MB (candle) to 25 MB (tract-onnx) to the
  program, which is the only cost worth noticing.
- **What decides it is pillar 1 and the Pack.** Networks are hard to explain, hard for a designer to tune, need random
  exploration to learn online, and must be retrained whenever their inputs or Actions change. Pack Actions change on
  every hot reload.
- **Recommendation: no deep network at runtime, in any phase planned so far.** In Track 1, two small, explainable uses
  are worth considering: Pack-bounded learned weights on considerations, and an offline tool that fits curves from
  playtest logs and writes them out as ordinary Pack data. Language goes to the LLM in Track 6.

## 1. Why LLMs are capable, and what transfers

| | Number | Source | Mark |
|---|---|---|---|
| GPT-3 | 175 B parameters | [Brown et al. 2020](https://arxiv.org/abs/2005.14165) | Summary |
| Chinchilla | 70 B parameters; "model size and the number of training tokens should be scaled equally" | [Hoffmann et al. 2022](https://arxiv.org/abs/2203.15556) | Summary |
| Llama 3 405B | 405 B parameters, 15.6 T tokens, 3.8×10²⁵ FLOPs, up to 16 K H100 GPUs | [Llama Team 2024](https://arxiv.org/html/2407.21783v3) | Read (numbers) |
| ChatGPT-style behaviour | Pretraining, then fine-tuning on human demonstrations and human rankings (RLHF) | [Ouyang et al. 2022](https://arxiv.org/abs/2203.02155) | Summary |

**Three things make an LLM capable, and a Pet's network would have none of them.**

1. **Data.** Pretraining reads a large share of the written internet. The model learns by predicting the next word, a
   signal available on *every word* of every document, so it never runs out of feedback.
2. **Parameters.** Billions of weights can store the patterns in that data. A small network can't, and doesn't need to.
3. **Pretraining, then adapting.** Almost all the knowledge comes from pretraining. Fine-tuning to a task uses
   comparatively little data because the model already "knows" most things.

**A network trained only on one owner's Pet.** **Inference**, from these assumptions:

- The Pet gets maybe **20 to 100 labelled moments a day**: petted, thrown, poked, played with. That is roughly 7,000 to
  36,000 a year. Ticks are plentiful (2.6 million a day at 30 Hz), but a tick isn't a lesson. Only the owner's reactions
  say "good" or "bad".
- A 32-64-16 network has **3,152 weights**. A common rule of thumb is ten or more examples per weight, about 30,000,
  or one to four years of one owner's play.
- **The signal is muddled.** The owner pets because they like the Pet, or because they're bored, not because of what it
  did a moment ago. Working out which earlier Action earned the reward is the hard "credit assignment" problem of
  reinforcement learning (RL). Creatures handled it with a decaying "susceptibility" (pet-memory.md §1). Candidate B
  keeps that through its cue window.
- **Compare game RL.** DeepMind's Atari agent used 50 million frames per game, about 38 days of play
  ([Mnih et al. 2015](https://www.nature.com/articles/nature14236), **Summary**). GT Sophy used far more (§2).

**What transfers is pretraining.** If the Pet ever needs world knowledge or language, borrow a model someone else
pretrained. That's the LLM in Track 6. Training our own from one owner's clicks won't get there.

## 2. Precedents

| System | What the network did | How it learned | Lesson for the Pet | Mark |
|---|---|---|---|---|
| **Creatures** (Grand et al. 1996, [paper](https://www.sussex.ac.uk/informatics/cogslib/reports/csrp/csrp434.pdf)) | ~1,000 neurons in 9 lobes, ~5,000 synapses per Norn. A Decision lobe picks a verb for the attended object. | Reward and Punishment *chemicals*, made when a drive goes down or up, change short-term weights. Long-term weights follow slowly. Stroke = reward, slap = punishment. | The closest ancestor of this project. The biochemistry is the explainable part: "hunger went down, so that was good". The design has already borrowed its two-speed weights. Grand budgeted 20,000 neurons a second for ten Norns, which shows how small it was. | Read |
| **Black & White** ([Wikipedia](https://en.wikipedia.org/wiki/Black_%26_White_(video_game)), citing Evans 2002 in *AI Game Programming Wisdom*; [postmortem](https://www.gamedeveloper.com/design/postmortem-lionhead-studios-i-black-white-i-)) | Belief-desire-intention. **Desires are simple perceptrons** (a weighted sum of a few inputs). **Opinions are decision trees** (how to satisfy a desire). | Slaps and strokes reinforce. The postmortem says the creature AI used no random elements. | Its "neural" part was a one-layer linear learner, like Rescorla-Wagner. Players read the learned *opinions* as personality. | Summary |
| **NEAT / rtNEAT in NERO** ([Stanley & Miikkulainen 2002](https://nn.cs.utexas.edu/downloads/papers/stanley.ec02.pdf); [Stanley et al. 2005](https://nn.cs.utexas.edu/downloads/papers/stanley.ieeetec05.pdf)) | Evolves both the weights and the shape of small networks. Each robot maps radar and rangefinder sensors to movement and fire. | 50 agents on the field. Every few ticks the worst is replaced by a child of two good ones. The player's **sliders are fitness weights**. 90 % of agents learned to seek an enemy in **99.7 s**. | It's fast because there are 50 learners and a fitness score every tick. One Pet has one body and sparse feedback. The paper also warns that RL's random exploration makes agents look erratic to players. | Read |
| **Supreme Commander 2** ([Robbins, Game AI Pro ch. 30](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter30_Using_Neural_Networks_to_Control_Agent_Threat_Response.pdf)) | Four MLPs, 34-98-15. **Each output is the expected utility of an Action**; flee if none is over 0.5. Under 0.03 ms per decision. | Offline self-play at full speed, taking random Actions and scoring them with a fitness function. About 1 hour per network, repeated several times after bugs. Frozen when shipped. | The closest fit to "a network as utility AI", and the author's own caveats apply here (§2.1). An "aggression" trait worked by skewing the inputs. | Read |
| **GT Sophy** ([Wurman et al. 2022, *Nature*](https://www.cs.utexas.edu/~pstone/Papers/bib2html-links/nature22.pdf)) | Policy and value networks with 4 hidden layers of 2,048 units, acting at 10 Hz. | Deep RL on 10 to 21 PS4s for 7 to 12 days, over 45,000 driving hours. Hand-tuned reward with 8 parts. Sportsmanship was "under-specified" and hard to encode. | Superhuman, and shipped in Gran Turismo 7 in 2023 ([GTPlanet](https://www.gtplanet.net/gt7-spec2-november-sophy-20231101/)). The cost is a research lab's. The hardest part was the *norms*, which is what "feels alive" is too. | Read (paper); Summary (GT7) |
| **Unity ML-Agents** ([Juliani et al. 2018](https://arxiv.org/abs/1809.02627); [repo](https://github.com/Unity-Technologies/ml-agents)) | Toolkit: deep RL and imitation learning trained outside the game, then run in Unity. | Offline, in simulation. | The standard pattern is to train offline and ship frozen. That means no learning from the owner. | Summary |
| **Ubisoft La Forge** ([Alonso et al. 2020](https://arxiv.org/abs/2011.04764); [interview 2022](https://www.ubisoft.com/en-us/studio/laforge/news/4PRxOnlOgGwEPcXYxZRQsq/ubisoft-la-forge-pushing-stateoftheart-ai-in-games-to-create-the-next-generation-of-npcs); [Roy et al. 2022](https://arxiv.org/abs/2112.12228)) | RL navigation and player bots. | Offline. Used for **testing** (Hyper Scape, For Honor). The interview says it was not yet in front of players. | Reward hacking in practice: a For Honor bot learned never to end the fight, and a navigation bot learned to bunny-hop. Their follow-up work tries to give designers back control through constraints. | Summary |
| **Forza Drivatar** ([Xbox Wire 2014](https://news.xbox.com/en-us/2014/09/30/games-forza-horizon-2-drivatars/)) | Learned driving profiles of real players, from Microsoft Research Cambridge. | Learns from watching each player drive, synced in the cloud since Forza 5. | A shipped "learns from you" system. It learns *how* you drive in one narrow skill, not what to want. | Summary |

### 2.1 Why learned NPCs are rare in shipped games

From the sources above (Robbins, Stanley, Romoff, Wurman), plus **Inference**:

1. **Designer control.** Robbins says the networks' biggest drawback is their black-box nature: designers can't make
   small adjustments. SupCom2 got away with it because campaign mode used separate, scripted AI.
2. **Any change means retraining.** Change the inputs, the outputs, the fitness function or the hidden size, and you
   retrain from scratch (Robbins). **Here, Actions and considerations are Pack data with hot reload**, so a network
   with one output per Action would go stale on every Pack edit.
3. **Testing.** You can't set a breakpoint on a weight. SupCom2's AI fled from the enemy commander because a fitness
   function didn't count winning. The cause was found only by reasoning about training. A creature that learns also
   multiplies the states to test, since each owner's Pet ends up different. **Inference.**
4. **Unpredictability.** Agents exploit the reward: For Honor, the bunny-hop, GT Sophy's aggression without collision
   penalties. Online learning also needs exploration, meaning occasional random Actions. That breaks pillar 1 (nothing
   random), and NERO's authors say players dislike exactly that.
5. **Cost and tooling.** Days of GPU time and a training pipeline per change, against an afternoon of editing curves.

## 3. Where a network could fit without hiding the "why"

| Option | What it is | Fits pillar 1? | Verdict |
|---|---|---|---|
| **a. Owner habits** (active hours, how they interact) | A small model of when the owner is around | Yes, but pet-memory.md's `Rhythm { owner_active[24] }` already is this model: a per-hour running average, 96 bytes, readable as a bar chart. A network adds nothing for one input (hour) and one output. | Keep the histogram. Add weekdays (168 bins) if wanted. |
| **b. Learned weight on a consideration** | Each Actor keeps a multiplier per consideration, nudged by the delta rule when an Action earns a reward ("petting after play": play's weight goes up) | Yes, if clamped to a Pack range, e.g. 0.5 to 2, and each change logs its cause like an association. It needs no random exploration: variety already comes without dice from memory's `since_last` and familiarity inputs (pet-memory.md, "Habits"), so rarely-won Actions still get tried. Only the chosen Action's weight updates, and the clamp bounds the drift. | **Possible in Track 1.** It's a one-layer linear learner, the same family as Rescorla-Wagner. The "why" reads "likes play more: petted after play ×12". |
| **c. Preferences from petting vs throwing** | Learn what the owner rewards | Candidate B already does it per subject, with two speeds and causes. An MLP would only add *conjunctions* ("likes petting in the evening, not when sleepy"). | Get conjunctions by keying associations on a context bucket (subject × time-of-day or mood band). That stays a table, not a network. |
| **d. Offline-trained policy in the Pack** | SupCom2-style: a frozen net scores Actions | Weakly. Attributions can show *which input* mattered, never *which episode* caused it. Pack edits invalidate it, and designers lose the curves. | **No** for the Pet. Revisit only for Encounter tactics (Track 3) if hand-tuned AI falls short. |
| **e. Design-time curve fitting** | A [Neural Additive Model](https://arxiv.org/abs/2004.13912) (Agarwal et al. 2021, **Summary**) learns one small curve per input and *adds* them. That is the shape of Infinite Axis considerations. Fit it offline on playtest logs ("did this feel right?") and export the curves as Pack TOML. | Fully. The runtime never sees a network, only curves a designer can read and edit. | **Optional Track 1 tool.** Zero runtime cost. Worth it only once there are logs to fit. |
| **f. Language intelligence** | The Track 6 LLM | Yes, if it *narrates and converses* using memory (landmarks, opinions) as context, and never picks per-tick Actions | **Track 6.** Cloud latency, cost, privacy and sampling randomness all rule it out of the 30 Hz loop. It could set slow inputs, such as a mood nudge after a chat, through a typed interface. |

**The web-app picture.** The utility Mind is like a rules engine whose rules live in config (the Pack). Candidate B is a
set of counters and running averages beside it. A deep network would be like replacing the rules engine with a trained
recommender: it can be better on average with huge data, but nobody can say why it chose this, and changing the config
means retraining. With one user's clicks as data, the recommender never gets good enough to pay for that.

## 4. Costs

### 4.1 CPU and RAM for a 32-64-16 MLP in plain Rust

About 40 lines, no dependencies, ReLU hidden layer, scalar loops the compiler vectorises. No `target-cpu=native`.
**Measured**, two runs each:

| | Per call | Per second at 30 Hz | Share of the ~5.5 ms/tick headroom (pet-memory.md) |
|---|---|---|---|
| Forward pass | **1.25 µs** | 38 µs | 0.023 % |
| Forward + update of the last layer (online learning) | 1.9 µs | 57 µs | 0.035 % |
| One pass per candidate Action (~11, BEH-06) | 14 µs | 0.41 ms | 0.25 % |
| Supreme Commander 2's 34-98-15, for comparison | < 30 µs on 2010 hardware, during an 8-player match (Robbins) | | |

The "30 Hz" column is the worst case. The draft in mind-architecture.md has the Mind decide on events and about every
0.5 s, not every tick, which would make it about 15 times cheaper.

RAM: 3,152 weights × 4 bytes = **12.6 KB** plus about 0.3 KB of activations per Actor. That is 0.03 % of the ~50 MB
left in the 150 MB budget. The test program was 160 KB in total.

### 4.2 Rust crates

Versions from crates.io on 2026-11-06. Sizes are the release `.exe` of a tiny program running the same 32-64-16 matmul
(tract: loading and running an ONNX file), built with the workspace's release profile. The program without a framework
is 160 KB. Inside the app, some code is shared with Bevy, so the real increase would be a little smaller. **Measured**,
unless marked otherwise.

| Crate | Latest | What it is | Trains? | `.exe` size | Per call (32-64-16) | Notes |
|---|---|---|---|---|---|---|
| [candle](https://github.com/huggingface/candle) (`candle-core`) | 0.11.1, 2026-10-31 | Hugging Face's minimal tensor library | Yes, through `candle-nn` | **0.92 MB** | ~5 µs, with reading the result back | Pure Rust. Built for LLMs, so a tiny net mostly pays overhead. |
| [burn](https://github.com/tracel-ai/burn) (`flex` backend) | 0.22.1, 2026-10-20 (0.23 in pre-release) | Full framework with many backends; `flex` is its pure-Rust CPU one | Yes | **1.9 MB** | ~2.1 µs, with reading the result back | Compile took 6 minutes. 0.22 changed `Tensor<B, D>` to `Tensor<D>` and deprecated the `ndarray` backend, so expect API churn. |
| [tract](https://github.com/sonos/tract) (`tract-onnx`) | 0.23.10, 2026-11-05 | Sonos' inference engine for ONNX/NNEF files | No | **25.2 MB** | not timed | Pure Rust. Train elsewhere, for example in PyTorch, and ship the file. Took 9 minutes to compile from clean. `tract-onnx` brings every ONNX operator. `tract-nnef` alone is likely smaller, but I didn't build it. |
| [ort](https://crates.io/crates/ort) | 2.0.0-rc.15, 2026-10-19 (still a release candidate) | Wrapper for Microsoft's ONNX Runtime 1.31 | Inference mainly | small wrapper **+ `onnxruntime.dll` 10.7–21.7 MB** | not timed | The DLL sizes are copies found on this machine (Windows, Office, Unreal); versions vary. DirectML adds ~10 MB. The Windows download is an 82.9 MB zip. |
| [dfdx](https://github.com/coreylowman/dfdx) | 0.13.0, **2023-07-27** | Typed tensors, checked at compile time | Yes | not built | | No release for over three years. Treat as unmaintained. |

**Clean-architecture placement.** If a runtime network is ever added, it belongs in `pet-core` as about 50 lines of
hand-written Rust: weights as data, no framework, no IO, deterministic, and testable with Given-When-Then. Frameworks
belong only in an offline tool, such as option e's curve fitter. That tool can depend on candle or burn without the app
shipping them.

### 4.3 Keeping the "why"

| Method | What it gives | Cost | What it can't give |
|---|---|---|---|
| Linear or additive model (options b, e) | **Exact**: each input's share is `wᵢ·xᵢ` or `curveᵢ(xᵢ)`, and the shares add up to the score | Free | Nothing lost |
| [Integrated Gradients](https://arxiv.org/abs/1703.01365) (Sundararajan et al. 2017, **Summary**) | Per-input shares for a deep net that add up to the difference from a baseline input | ~20–50 forward and backward passes, under ~0.2 ms, only when the tooltip opens. **Inference.** | A *cause in time*. It says "opinion(cursor) mattered most", not "because of 3 throws 4 minutes ago". |
| [SHAP](https://arxiv.org/abs/1705.07874) (Lundberg & Lee 2017, **Summary**) | Similar shares based on game theory | More passes than IG | The same |

The gap in the last column is the real cost. Candidate B ties every number to the episodes that caused it. A hidden
layer mixes inputs so that the trail from "this weight" back to "that evening" is lost. Attributions recover which
input mattered, but not the story.

## 5. Recommendation

| Phase | Network? | What to do |
|---|---|---|
| **MVP (roadmap item 4)** | None beyond Rescorla-Wagner | Build candidate B as planned. Say plainly that its opinions are a one-layer learner. `pet-core` already emits a "why" record per decision. A writer outside `pet-core`, which has no IO, could log it whenever the chosen Action changes. That log is free training data for later. |
| **Track 1, Behaviour** | Linear and additive only | (1) Option b, learned consideration weights clamped by the Pack, if the Pet feels too fixed after play-testing. (2) Option e, the offline curve fitter, once there are decision logs and owner ratings. Neither puts a hidden layer in the runtime. |
| **Track 3, Encounters** | Maybe offline, for enemies | Only if hand-tuned tactics fall short, SupCom2-style, frozen, and never for the Pet's own Mind. |
| **Track 6, AI companion** | A pretrained LLM, not our own | Language, conversation and narrating the "why" from memory. Kept out of the 30 Hz loop. |

**When to revisit.** Consider a runtime MLP only if play-testing shows a gap that tables can't close: behaviour that
needs many interacting inputs, after context-keyed associations have been tried. Even then, start with option e and see
whether fitted curves are enough.

## 6. Decisions for the owner

1. **Is the "why" a hard rule?** If every Action must show a cause traced to episodes, deep networks are out of the
   runtime for good. If "these inputs mattered most" is enough, a frozen MLP becomes possible later.
2. **May the Pet's weights drift through learning (option b)?** If so, within what Pack range? Can the owner reset
   them? Is drift shown in the dev panel?
3. **Should decisions be logged from the MVP on?** Logging only when the Action changes means perhaps 500 to 3,000 rows
   a day at about 100 bytes each, so 50 to 300 KB a day. **Inference.** It would be kept locally and never sent
   anywhere. It is the only
   way to get data for option e. It may need the same Settings switch as the Rhythm (pet-memory.md decision 6).
4. **Contextual opinions.** Are subject × context buckets (time of day, mood band) worth the save size before
   anything heavier is considered?
5. **Track 6 boundary.** May the LLM change slow Mind inputs (mood, opinions) after a conversation, or only read them?
6. **Encounter AI.** Is a frozen, offline-trained enemy policy acceptable in Track 3, given that designers lose
   fine-grained tuning of it?
