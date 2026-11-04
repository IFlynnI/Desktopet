# firstborn, stage 1: three character concepts

**Gate 1: passed on 2026-11-03. The owner picked A, Brim.** Previews:
- [previews/01-concepts.html](previews/01-concepts.html) shows each concept at 1×, at 2× on the taskbar, on light and
  dark apps, and as a silhouette. It works when opened straight from disk.
- [previews/01-concepts.png](previews/01-concepts.png) is a contact sheet.

The thumbnails are rough. They are there to judge the idea, the silhouette and the colour, not the final drawing. Each
one is an egg plus a hatchling in its signature pose, at roughly the MVP size: the hatchlings are about 50 × 55 px.

`firstborn` is a working Pack id, so the folders needn't move whichever concept wins. One side effect: the app uses the
first Pack by folder name, and `firstborn` sorts before `placeholder`, so with both installed, this one loads.

All three fit the MVP Form graph: egg, then the hatch, then a hatchling.

---

## A. Brim, the helmeted brimstone drake *(recommended)*

| Field | Content |
|---|---|
| **Name / species** | Brim. A brimstone drake: a small fire dragon. |
| **One-line pitch** | A pocket-sized knight of a dragon who never took off the top of its eggshell, and wears it as a helmet. |
| **Silhouette test** | Passes at 1×. The key features are a big domed helmet with two horns punched through it, a zigzag brow where the shell broke, a heavy ember-tipped tail and stubby wings. In black, it reads as "acorn with horns", which nothing else on a desktop looks like. |
| **Palette** | 20 colours. Slate-blue body, amber belly plates and wings, warm cream shell with lilac shadows, bone horns, and glowing gold eyes. The ember red and gold appear only on the tail flame and eyes, so they draw the eye. The body is mid-value with a dark outline, so it reads on both light and dark apps. |
| **Personality traits** | **Bold**: it is quick to approach and pounce on the cursor, and its wariness after a throw fades fast. That's a high boldness trait and a short memory of being thrown. **Show-off**: it does a "ta-da" pose after a good landing or pounce, and sometimes idles in its knight stance. That's a high playfulness gain from successful pounces. **Secret softie**: it acts tough, but when petted the helmet slides back and its eyes go from glowing slits to big round eyes. That's a high sociability gain from petting. It looks how it acts: armour on the outside, a baby underneath. |
| **Signature pose** | "Ready!": helmet tipped up with one claw, chest puffed, tail flame flicked high, a smoke puff from the nose. |
| **Signature idle** | Breathing, with the helmet slowly slipping down over its eyes until it pushes it back up. This is the idle variant you'll notice and smile at. |
| **Forms** | Egg, then hatchling (wears the shell top as a helmet), then juvenile (the helmet has cracked into a ring, like a crown around the horns), then adult (one shell shard worn as an amulet or pauldron). The shell tells the growth story, which fits pillar 4 ("Grows with you"): you can see how far it has come by what is left of the egg. |
| **Transformation sequences** | **Hatch:** the egg wobbles, then the zigzag seam glows. The top pops off and flips in the air while the hatchling blinks out. The top lands crooked on its head and it pushes the shell straight. The egg's seam is the same zigzag as the helmet's brow, so the moment pays off the design. **Ultra mode (later):** the helmet's crack burns gold and the eyes blaze. |
| **Core animation set** | idle (with helmet-slip and smoke-puff variants), walk (a waddle that makes the helmet bob), sit, sleep (pulls the helmet down over its face like a hat), look-at, follow/pounce, fall (holds the helmet on with both claws), land (the helmet clonks down over its eyes), poke reaction (helmet "tink"), petted, held. For the MVP egg: idle and wobble, plus the hatch sequence. |
| **Extended animations** | Polishing the helmet, tapping it like a drum, sneezing a spark, using the helmet as a bowl. |
| **Movement mode per form** | Hatchling and juvenile walk. The adult walks and glides. |
| **Sound palette** | A ceramic "tink" from the shell, a kettle-like purr, a small growl-chirp, an ember crackle, and a hollow "clonk" on landing. |

**Why it's the strongest:**
- The helmet is a prop that is also part of the body, so it does three jobs at once.
  - It gives a silhouette no other mascot has.
  - It's a way to show feelings without a face full of detail. That matters at 50 px, where an eye is 3 pixels.
  - It ties the hatch to the character for its whole life.
- It's cute, because it's a baby in an eggshell. It's cool, because it's a knight's helm with a glowing visor.
- It's cheap to animate. The helmet is a rigid shape that moves as one piece, so it stays consistent across Frames.

**Risk:** the helmet makes the head heavy. Every pose has to keep it balanced, or the Pet looks top-heavy and tips over.

---

## B. Nox, the crescent night drake

| Field | Content |
|---|---|
| **Name / species** | Nox. A night drake. |
| **One-line pitch** | A slinky, cat-tempered dragon who's all cool indifference by day and a little comet after dark. |
| **Silhouette test** | Passes at 1×, but only just. The key features are two sweeping crescent-moon horns and a long tail held in a question-mark curl. The horns can't get thinner than 2 px, or they break up at 1×. |
| **Palette** | 18 colours. Midnight navy body, lavender belly, moon-gold horns, white and cyan star freckles, pale gold slit eyes. **Risk:** the dark body nearly vanishes on dark apps and the dark taskbar (see the dark preview). It would need a lighter rim colour. |
| **Personality traits** | **Aloof**: low sociability gain at first, and petting has to be earned. After a while, it leans in. **Nocturnal**: its energy follows the time of day (BEH-03), sleepy and slow in the morning, perky and zoomy after sunset. **Stalker**: cursor play is a slow crouching stalk, then a pounce, like a cat. |
| **Signature pose** | Sitting tall with its tail wrapped round its feet and the horns catching a glint. |
| **Signature idle** | The tail tip flicks, and the stars on its back twinkle one after another. |
| **Forms** | Egg (dark, with a constellation and a crescent), then hatchling, then juvenile (longer horns), then adult (its wing membranes show a galaxy). |
| **Transformation sequences** | **Hatch:** the constellation lights up star by star, then the shell dissolves into falling stars. **Ultra mode:** an eclipse, where the body goes black inside a corona. |
| **Core animation set** | The same list as A. The stalk-pounce is its standout. |
| **Extended animations** | Chasing its own tail, knocking things off the taskbar edge, a star yawn. |
| **Movement mode per form** | Walk (a slink), and later glide. |
| **Sound palette** | Soft chimes, a purr, a tiny roar that cracks. |

**Strengths:** it's cool-first and very elegant, and the night routine makes "Alive" visible without any extra mechanics.
**Weaknesses:**
- Its readability is the worst of the three: a dark body, and thin horns at 1×.
- "Aloof" works against "Better together" for the first days.
- The cat-dragon idea has been done more often.

---

## C. Volt, the storm axolotl *(wildcard)*

| Field | Content |
|---|---|
| **Name / species** | Volt. A storm axolotl, a salamander. In folklore salamanders are elemental cousins of dragons, and Volt grows into a storm wyrm. |
| **One-line pitch** | A grinning puddle of static with six lightning-rod gills. |
| **Silhouette test** | Passes easily at 1×. The key feature is a starburst of zigzag gill fronds around a wide, flat head. It is the most distinctive silhouette of the three. |
| **Palette** | 13 colours, room to grow to 18. A soft periwinkle-cyan body, violet-to-magenta gills with yellow spark tips, pink cheeks. It's bright and friendly, and readable everywhere. |
| **Personality traits** | **Hyper**: high energy, and an activity setting that shows the most. **Giggly**: poke combos stay at the giggle tier much longer (INT-04). **Jumpy**: when startled, its gills spark and puff out, and it stays wary of the cursor for a short while. |
| **Signature pose** | Gills flared with sparks at every tip and a big open grin. |
| **Signature idle** | The gills sway like seaweed, and now and then a spark runs from tip to tip. |
| **Forms** | Jelly egg (you can see it curled up inside), then larva (the hatchling), then juvenile, then storm wyrm (legs shrink and it starts to fly). So it still becomes "the dragon", but late. |
| **Transformation sequences** | **Hatch:** a static build-up, a zap, and it splashes out grinning. **Ultra mode:** thunderhead, with the gills turning into lightning. |
| **Core animation set** | The same list as A. The petted and poke reactions are its standout (the gills react). |
| **Extended animations** | Blowing bubbles, a zap at the cursor, a puddle flop. |
| **Movement mode per form** | Walk (a wiggle). The storm wyrm hovers. |
| **Sound palette** | A crackle, a squeaky chirp, a "bloop". |

**Strengths:** the easiest silhouette to read, the cutest face, and a very clear personality.
**Weaknesses:**
- It isn't a dragon at hatch time, which goes against the PRD's first character.
- It is more cute than cool.
- Six gills are six moving parts to animate in every Frame. That costs the most of the three.

---

## The three side by side

| | A. Brim | B. Nox | C. Volt |
|---|---|---|---|
| Silhouette at 1× | strong, one of a kind | fair, thin horns | strongest |
| Readable on dark apps | yes | weak | yes |
| Cute ↔ cool | balanced | cool | cute |
| Fits "dragon from an egg" | yes, and the egg stays part of it | yes | late, through evolution |
| Ways to show feeling at 50 px | helmet and eyes | tail and stars | gills |
| Cost to animate | low (rigid helmet) | medium | high (six gills) |

**Recommendation: A, Brim.** Pieces of the others could carry over if you like them. Nox's time-of-day routine is
behaviour, not art, so any character can have it. Volt's "a spark runs along the edge" idle could become Brim's
tail-ember flicker.
