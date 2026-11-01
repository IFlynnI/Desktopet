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

**Hidden**:
The Pet's state while a Manual hide or an Auto-hide applies. Never used for being absent from the taskbar
or Alt-Tab, which is simply how the Overlay always is.
_Avoid_: Invisible, minimised, paused
