# The Mind learns from the owner and from its own Experiments, and every choice has a named reason

The owner wants a Mind that understands its surroundings, learns fast, remembers for a long time, and combines what
it can do in ways nobody wrote down beforehand. So capabilities emerge from general Actions and general mechanics,
and no Action or rule exists only to make one scenario work. A smash's damage comes from the physics fall speed, for
example, not from a special rule for smashing after a throw. Every choice must trace back to named causes, which can
be:
- drives;
- memories;
- learned Links ("after this, in this context, that tends to follow");
- an Experiment.

Experiments use dice from a seed saved with the Pet, so a test or a replay gives the same result. They only pick
options that score at least half as well as the best, they fade as the Pet learns, they never run during a focus
session, and they are always shown with ❓. Pillar 1 changes from "nothing it does is random" to "when it experiments
it shows it (❓), its experiments stay within limits and fade as it learns, and nothing it does goes unexplained".

The hard "why" rule keeps deep neural networks out of the Mind for now. A small network costs nothing: a 32-64-16 MLP
takes 1.25 µs a tick in plain Rust ([neural-mind.md](../research/neural-mind.md)). The reasons are elsewhere:
- **Explanation:** a network can't say why it chose something.
- **Hot reload:** every Pack edit would mean retraining it.
- **Data:** one Pet sees perhaps 20–100 meaningful moments a day, so even that small network would need an estimated
  1–4 years of one owner's play. Llama 3 learned from 15.6 trillion tokens.

The memory's own learning rule is already a one-layer learner whose causes can be named. Learning stays small instead:
- each learnable Action reads at most a few Features, split into a few ranges;
- the first try sets a cell outright, and later tries adjust it;
- rewards are the "how pleasant" value memory already gives each event.

So one rewarded try can already change a choice ([teachable-mind.md](../research/teachable-mind.md)). We revisit deep
networks if the Pet's intelligence falls short of what the owner hopes for.

## Consequences

- The MVP ships:
  - learned Links;
  - Experiments;
  - the Marker, a middle click whose worth the Pet learns from the petting that follows it;
  - decision records behind every "why";
  - one shared list of Features;
  - a local decision log.

  Track 1 adds the planner that chains Links into Routines, which is how multi-step combinations appear.
- Everything learned is part of the Character's Progress and is saved with it. It stays within limits the Pack sets.
  The owner can see it link by link in the Pet panel, and erase any of it.
- Learned Links are keyed by Action. When the Pet evolves, what it knows about Actions the new Form shares carries
  over, and the rest goes dormant.
- Asking the owner for something, such as a throw, is part of a separate communication system: Emotes now, the AI track
  later. It is not built into learning.
- New Actions are added only as general building blocks that are useful in their own right, and each is checked
  against the emergence principle.
