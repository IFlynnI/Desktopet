"""The poses shown on the gate 2 style guide. Shared by build_style.py and the paint-over."""
import brim_rig as R

P = R.pose

NEUTRAL = P()
READY = P(eye="visor", mouth="fang", slip=-1, helmet_rot=-5, arm="fist", tail_lift=8, wing=1, puff=(0, 0), sy=1.03)

EXPRESSIONS = [
    ("visor: cool, default", P(eye="visor")),
    ("round: curious, cute", P(eye="round", slip=-2, mouth="o")),
    ("half: bored, sleepy", P(eye="half", slip=2, mouth="flat")),
    ("happy: petted", P(eye="happy", mouth="grin", blush=True, slip=-1)),
    ("wide: startled", P(eye="wide", mouth="open", slip=-3)),
    ("angry: annoyed", P(eye="angry", mouth="flat", slip=1, helmet_rot=4)),
    ("squint: smug", P(eye="squint", mouth="fang")),
    ("closed: asleep", P(eye="closed", mouth="none", slip=3)),
]

HELMET = [
    ("on, level", P()),
    ("slipped over the eyes", P(slip=7, mouth="flat")),
    ("tipped up", P(slip=-3, eye="round", mouth="smirk")),
    ("crooked", P(helmet_rot=12, helmet_dx=-1, eye="half", mouth="flat")),
    ("off: bare head", P(helmet_off=True, eye="round", mouth="o")),
]

EXTREMES = [
    ("pounce, mid-air", P(lift=-14, lean=-8, sx=1.12, sy=0.92, front_foot=(5, -1), back_foot=(-7, -1), arm="reach",
                          tail_lift=-3, wing=2, eye="wide", mouth="open")),
    ("held, dangling", P(sy=1.1, sx=0.92, front_foot=(1, 0), back_foot=(-1, 0), arm="hold", eye="angry", mouth="flat",
                         tail_lift=-6, slip=1)),
    ("sleep curl (rough)", P(sy=0.72, sx=1.12, head_dy=6, head_dx=1, slip=4, eye="closed", mouth="none",
                             tail_wrap=True, nostril=False)),
    ("hatch peak", P(helmet_off=True, sy=0.95, eye="wide", mouth="open", arm="up", shell_bottom=(32, 52, 0),
                     shell_top=(27, 27, -150))),
]
