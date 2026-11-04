"""The poses shown on the gate 2 style guide (version 2, the 3D rig)."""
import brim as R

P = R.pose

NEUTRAL = P()
READY = P(mouth="fang", rock=8, near_hand=(15.0, 13.0, 9.0), tail_lift=5, wings=1, puff=(0, 0), sy=1.03)

EXPRESSIONS = [
    ("visor: cool, default", P(eye="visor")),
    ("round: curious, cute", P(eye="round", rock=6, mouth="o")),
    ("half: bored, sleepy", P(eye="half", rock=-6, mouth="flat")),
    ("happy: petted", P(eye="happy", mouth="grin", blush=True, rock=4)),
    ("wide: startled", P(eye="wide", mouth="open", rock=10)),
    ("angry: annoyed", P(eye="angry", mouth="flat", rock=-4, helmet_roll=4)),
    ("squint: smug", P(eye="squint", mouth="fang")),
    ("closed: asleep", P(eye="closed", mouth="none", rock=-8)),
]

HELMET = [
    ("on, level", P()),
    ("slipped over the eyes", P(rock=-34, mouth="flat")),
    ("tipped up", P(rock=18, eye="round", mouth="o")),
    ("crooked", P(helmet_roll=10, rock=-5, eye="half", mouth="flat")),
    ("off: bare head", P(helmet_off=True, eye="round", mouth="o")),
]

# The extreme poses come from the approved timing sheets, so the cell proof stays true.
import os as _os
import sys as _sys
_sys.path.insert(0, _os.path.join(_os.path.dirname(_os.path.abspath(__file__)), "..", "anim"))
import hatchling as _H  # noqa: E402

_A = {a.name: a for a in _H.ANIMATIONS}
EXTREMES = [
    ("pounce, mid-air", _A["pounce"].frames[6].pose),
    ("held, dangling", _A["held"].frames[2].pose),
    ("sleep: dozing, hunched", P(**_H.SLEEP)),
    ("hatch peak", _A["hatch"].frames[9].pose),
]
