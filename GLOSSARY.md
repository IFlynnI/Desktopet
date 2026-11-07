# Desktop Pet

A desktop companion that lives on top of the owner's other Windows apps. This glossary fixes the words
used for it in code, docs and conversation.

## Language

### On the desktop

**Pet**:
The living character the owner sees and interacts with.
_Avoid_: Sprite (that is its art), mascot, buddy

**Overlay**:
The transparent, always-on-top surface the Pet is drawn on. The Pet lives in the Overlay; they are not
the same thing.
_Avoid_: Pet window, canvas

**Home monitor**:
The one monitor the Pet lives on.
_Avoid_: Active monitor, current screen

**Ground plane**:
The line along the top of the taskbar that the Pet stands on.
_Avoid_: Floor, baseline

**Hit mask**:
The set of pixels in one frame of art that catch clicks. Everywhere else, clicks pass through to the app
underneath.
_Avoid_: Hitbox, collision mask

### Showing and hiding

**Presence**:
Whether the Pet is shown or Hidden, and which hides currently apply.
_Avoid_: Visibility, show state

**Manual hide**:
A hide the owner asked for from the hotkey or the tray.
_Avoid_: Bare "hide", user hide

**Auto-hide**:
A hide caused by a fullscreen application being in the foreground.
_Avoid_: Bare "hide", fullscreen hide

**Return delay**:
How long the Pet waits before coming back when an Auto-hide ends, so a quick Alt-Tab needn't show it.
_Avoid_: Debounce, show delay

**Hidden**:
The Pet's state while a Manual hide or an Auto-hide applies. Never used for being absent from the taskbar
or Alt-Tab, which is simply how the Overlay always is.
_Avoid_: Invisible, minimised, paused

**Relaunch**:
The app starting itself again after a render failure. Unlike a launch by the owner, it keeps a Manual hide.
_Avoid_: Restart, respawn

### Characters and art

**Pack**:
One folder holding everything for one Character: its art and its data. The app ships with no Character of its
own; each one comes from a Pack.
_Avoid_: Content pack, character pack, skin, theme

**Character**:
The identity a Pack describes, such as the dragon. It has one or more Forms.
_Avoid_: Pet (that is the living thing on the desktop), species

**Form**:
One stage of a Character, such as egg or hatchling. A Form brings its own art and can also change how the Pet
behaves.
_Avoid_: Stage, skin, costume

**Animation**:
A named sequence of Frames that a Form can play, such as idle or walk.
_Avoid_: Clip, state

**Frame**:
One picture in an Animation, with its own duration. A Hit mask is built from one Frame.
_Avoid_: Cel, image

**Action**:
Something the Pet can decide to do, such as walk or sleep. A Form can add Actions of its own.
_Avoid_: Behaviour, move, state

**Situation**:
Something that happens to the Pet rather than something it chooses, such as being held, falling or landing. Each
Form says which of its Animations to play for each Situation, and while one applies, the Pet can still choose Actions
that allow it.
_Avoid_: Built-in Action, state, Reaction (that answers a Gesture)

**Evolution**:
A permanent change from one Form to another. Once its conditions are met it waits for the owner to choose it, and it
is never automatic.
_Avoid_: Level-up, transformation

**Transformation**:
A temporary change into another Form that ends by itself, such as the dragon's ultra mode in a fight.
_Avoid_: Evolution, power-up

**Anchor**:
The point where the Pet's feet meet the ground. Where the Pet is means where its Anchor is, and it turns around this
point.
_Avoid_: Pivot, origin, position

**Emote**:
A small piece of animated art shown above the Pet to make its motive visible, such as 💤 when tired. Emotes belong to
the Character, not to a Form. Each one either plays once or stays until the motive passes.
_Avoid_: Bubble, icon, particle

### Moving and fighting

**Actor**:
Anything on the desktop that moves, acts and can be hurt: the Pet, or an enemy. Each Actor has a Mind.
_Avoid_: Entity, unit, character

**Mind**:
What decides an Actor's next Action. Minds come in kinds, from a simple one for a basic enemy to the Pet's own, which a
boss can also use.
_Avoid_: AI, brain, controller

**Projectile**:
Something fired that moves and strikes but has no Mind and can't be hurt, so it isn't an Actor.
_Avoid_: Bullet, missile

**Body**:
The rectangle an Actor takes up in the world, used for landing on the Ground plane and bouncing off screen edges. Never
used for clicks or combat.
_Avoid_: Collider, bounds, Hit mask

**Obstacle**:
A solid rectangle in the world that Bodies collide with and can stand on. The Ground plane and the screen edges are the
first ones the Pet meets.
_Avoid_: Wall, platform, collider

**Material**:
How grippy and how bouncy a surface or a Body is. Each Form gives its Body one, and an Action or a Situation can
override it while it runs, such as a landing that never bounces.
_Avoid_: Physics material, surface type

**Prop**:
A thing in the world that can be thrown, hit or stood on but has no Mind, such as a ball.
_Avoid_: Toy, item, object

**Hurt box**:
The rectangle where an Actor can be struck.
_Avoid_: Hitbox

**Strike box**:
The rectangle an attack, a Projectile or a thrown Pet hits with. It only counts while the attack is live, and each
target is struck at most once per attack.
_Avoid_: Hitbox, attack box

### The Mind

**Drive**:
An inner urge that rises and falls by itself, such as energy or playfulness, and makes some Actions more appealing.
The owner never has to tend one.
_Avoid_: Need, stat, meter

**Trait**:
A lasting side of a Character's personality, such as playful or bold, set by its Pack. Traits don't change from moment
to moment; they tilt every choice the Mind makes.
_Avoid_: Stat, attribute, personality setting

**Feature**:
A named reading of the world that the Mind can notice and learn about, such as how high the Pet is or whether the cursor
is near.
_Avoid_: Sensor, input, variable

**Link**:
Something the Pet has learned: after an Action in a certain context, a certain result tends to follow.
_Avoid_: Association (that is an opinion of a subject), rule, edge

**Experiment**:
A choice the Pet makes to try something it knows little about. It is always shown to the owner and stays within limits.
_Avoid_: Random action, exploration

**Routine**:
A multi-step plan the Pet has discovered by chaining Links.
_Avoid_: Habit, combo, script

**Landmark**:
A moment the Pet remembers for good because it stood out, such as its first throw or a personal best. Only a few are
kept, and the weakest makes way for a stronger one.
_Avoid_: Highlight, milestone, core memory

**Rhythm**:
What the Pet has learned about when the owner is usually around, hour by hour. The owner can see it, reset it or turn
it off.
_Avoid_: Schedule, habit, Routine (that is a plan the Pet discovered)

### The owner and the Pet

**Gesture**:
Something the owner does to the Pet with the pointer, such as a poke, a stroke, or a drag and throw.
_Avoid_: Input, click, interaction

**Marker**:
A Gesture that says "that one, right then" without being a reward itself. The Pet learns what it is worth by noticing
what follows it, the way a clicker works in animal training.
_Avoid_: Clicker, reward, confirm

**Reaction**:
What the Pet does in answer to a Gesture, such as giggling when poked. Each Form says which of its Actions answers each
Gesture.
_Avoid_: Response, reflex, Situation (what happens to the Pet, not what it does in answer)

**Pet panel**:
The separate window where the owner sees what the Pet is thinking and why, tunes it and changes its settings. Once
closed, nothing of it stays in the Pet.
_Avoid_: Settings window, dashboard, inspector, dev panel

### Domain

**Definition**:
The read-only description of something, which comes from a Pack and never changes while the app runs.
_Avoid_: Config, template, blueprint

**Progress**:
What the Pet has become and remembers, kept per Character so that switching Characters never loses it.
_Avoid_: Save data, state

**XP**:
Experience the Pet earns from what happens to it, such as being petted or hatching. How much each kind of event earns
can be tuned, and XP is never lost.
_Avoid_: EXP, points, score

**Level**:
How far the Pet has grown, worked out from its total XP. It never drops, and reaching one can open an Evolution.
_Avoid_: Rank, tier, stage

**Ledger**:
The Pet's record of all the XP it has earned, in total and by kind of event. It only ever grows.
_Avoid_: XP bar, balance, wallet
