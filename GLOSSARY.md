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
Form says which of its Animations to play for each Situation.
_Avoid_: Built-in Action, state, reaction

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

**Hurt box**:
The rectangle where an Actor can be struck.
_Avoid_: Hitbox

**Strike box**:
The rectangle an attack, a Projectile or a thrown Pet hits with. It only counts while the attack is live, and each
target is struck at most once per attack.
_Avoid_: Hitbox, attack box

### Domain

**Definition**:
The read-only description of something, which comes from a Pack and never changes while the app runs.
_Avoid_: Config, template, blueprint

**Progress**:
What the Pet has become and remembers, kept per Character so that switching Characters never loses it.
_Avoid_: Save data, state
