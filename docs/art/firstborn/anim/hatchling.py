"""The hatchling Form's Animations: timing sheets with their key poses.

Ticks are 30 fps ticks (33.3 ms). K = key, I = in-between, S = smear, H = hold.
At gate 3 only keys and smears have poses; in-betweens are posed in stage 4.
"""

from timing import Anim, H, I, K, S
import brim

P = brim.pose

NEAR_REST = (8.5, 10.5, 8.5)
FAR_REST = (8.0, 11.0, -7.0)
NEAR_BROW = (10.0, 33.0, 13.0)     # the near hand at the helmet's brow, in front of the near cheek
FAR_BROW = (11.0, 34.0, -7.5)

REST = P()

# How far below the ground a lone far-foot plant is drawn so it shows on the Ground plane's row:
# the far foot is 11 px further back, which shows 2.3 px higher with the camera 12 degrees above.
FAR_PLANT = -2.3


def walk_frame(phase):
    """One of the 8 walk drawings. phase 0-7: contact, down, passing, up, for each foot.
    The planted foot moves back 1.5 px per drawing, and the swinging foot moves forward and lifts,
    so at 15 px/s in model space (13 px/s on screen at 1x) the planted foot doesn't slide."""
    step = phase // 4
    q = phase % 4
    plant = 3.0 - 1.5 * q                      # the foot on the ground goes back: 3, 1.5, 0, -1.5
    swing = -3.0 + 1.5 * q                     # the other comes forward
    lift_swing = [0.0, 1.0, 2.0, 1.0][q]
    body = [0.0, -1.0, 0.0, 1.0][q]            # contact, down, passing, up
    near = (plant, 0.0) if step == 0 else (swing, lift_swing)
    # The far foot's contact shows higher on screen than the near foot's, so when it's the only foot
    # down it's drawn a little lower, to stay on the Ground plane's row.
    far = (swing, lift_swing) if step == 0 else (plant, FAR_PLANT if lift_swing > 0 else 0.0)
    arm = 2.0 if step == 0 else -2.0           # arms swing against the legs
    arm = arm * [1.0, 0.6, 0.0, -0.6][q]
    rock = [0.0, -3.0, -1.0, 2.0][q]           # the helmet lags the bob by a drawing
    return P(near_foot=near, far_foot=far, torso_drop=-body, sy=[1.0, 0.98, 1.0, 1.01][q],
             near_hand=(NEAR_REST[0] - arm, NEAR_REST[1], NEAR_REST[2]),
             far_hand=(FAR_REST[0] + arm, FAR_REST[1], FAR_REST[2]),
             rock=rock, tail_swing=(3.0 if step == 0 else -3.0) * [1.0, 0.7, 0.0, -0.7][q])


# Sitting on the haunches: the upper body drops onto the thighs, which point forward along the
# ground; the feet stand up in front; the tail lies on the ground behind and curls towards you.
SIT = dict(legs="sit", torso_drop=2.0, torso_tilt=-9.0, near_hand=(9.0, 7.5, 9.5), far_hand=(9.0, 8.0, -8.0),
           tail_root=True, tail_path=[(-7.0, 6.0, 0.0), (-14.0, 4.3, -1.0), (-19.5, 3.4, 0.0), (-22.0, 2.6, 3.0),
                                      (-19.5, 2.2, 5.0)])
SIT_TAP = dict(SIT, tail_path=SIT["tail_path"][:-1] + [(-19.5, 5.5, 5.0)])

# Dozing off sitting up: hunched forward, head drooping, the helmet slid down over the face.
SLEEP = dict(SIT, torso_tilt=16.0, head_dy=-3.0, head_nod=24.0, rock=-24.0, eye="closed", mouth="none",
             near_hand=(10.0, 8.0, 6.0), far_hand=(10.0, 8.0, -4.5), flame_kind="dim")

# The lower shell's four quarters, as (from angle, to angle) round the egg: far side, front, near
# side, back. Each falls away from Brim, tipping over onto the ground.
SHARDS = [(-135, -45), (-45, 45), (45, 135), (135, 225)]


def shards(stage):
    """The pieces at one moment of the burst: 0 just cracked, 1 falling outward, 2 lying on the ground.
    The front piece tips least, so it stays inside the cell."""
    tip = [(6, 4, 6, 6), (35, 20, 30, 30), (70, 40, 60, 60)][stage]
    out = [0.4, 0.8, 0.8][stage]
    # every piece rests on the Ground plane: it tips over its bottom edge, it doesn't float
    return tuple((a0, a1, out, None, t, 0.0) for (a0, a1), t in zip(SHARDS, tip))


def run_frame(phase):
    """One of the 6 run drawings: contact, push-off, flight, for each foot. The planted foot moves
    back 2.67 px per drawing on twos: 40 px/s in model space, 35 px/s on screen at 1x."""
    step, q = phase // 3, phase % 3
    plant = [3.0, 0.33, -3.0][q]
    swing = [-4.0, 0.0, 3.0][q]
    # a planted foot is lifted 0.8 px against the forward lean, which tips the front of the foot down
    plant_lift, swing_lift = [(1.9, 2.0), (0.8, 3.0), (2.0, 2.5)][q]
    near = (plant, plant_lift) if step == 0 else (swing, swing_lift)
    far = (swing, swing_lift) if step == 0 else (plant, plant_lift + (FAR_PLANT if q < 2 else 0.0))
    arm = 3.0 if step == 0 else -3.0
    return P(near_foot=near, far_foot=far, lift=[0.0, 0.0, 3.0][q], torso_drop=[0.0, -1.0, 0.0][q],
             sy=[0.97, 1.0, 1.04][q], lean=-10,
             near_hand=(8.5 - arm, 12.0, 9.0), far_hand=(8.0 + arm, 12.5, -7.0), rock=[-3.0, 0.0, 4.0][q],
             tail_lift=-2, tail_swing=3.0 if step == 0 else -3.0, eye="round", mouth="grin", wings=0.3)


ANIMATIONS = [
    Anim("idle", [
        K("rest", 15, REST),
        I("breathing in: the chest swells", 6, P(chest=0.02)),
        K("full breath: head and helmet up 1 px", 15, P(head_dy=1.0, chest=0.03)),
        I("breathing out", 6, P(chest=0.02)),
        K("rest", 21, REST),
        K("blink: half", 2, P(eye="half")),
        K("blink: closed", 3, P(eye="closed")),
        I("blink: half", 2, P(eye="half")),
        H("rest", 30, REST),
    ], note="The default Action. Slow breathing on long holds, one blink. The flame flickers throughout."),

    Anim("idle_helmet", [
        K("rest", 15, REST),
        K("the helmet creeps forward", 30, P(rock=-10, eye="half")),
        K("further: Brim squints", 30, P(rock=-20, eye="squint", mouth="flat")),
        K("over the eyes: a comic beat", 20, P(rock=-32, mouth="flat")),
        K("anticipation: head dips, arm drawn back", 6, P(rock=-32, head_nod=6, sy=0.97, mouth="flat",
                                                          near_hand=(6.0, 9.0, 9.0))),
        S("smear: the arm swipes up and the helmet starts back", 2,
          P(rock=-8, head_nod=2, near_hand=(11.0, 27.0, 13.5), mouth="o")),
        K("overshoot: the helmet tips back, eyes round", 3, P(rock=14, eye="round", mouth="o",
                                                              near_hand=(11.0, 36.0, 13.0))),
        I("follow-through: it settles forward", 3, P(rock=-4, eye="round", near_hand=(10.0, 28.0, 13.0))),
        I("and back", 3, P(rock=2)),
        K("smug: squint and fang", 20, P(eye="squint", mouth="fang")),
        H("rest", 15, REST),
    ], once=True, note="An idle variant. The helmet slips over the eyes, and Brim shoves it back up."),

    Anim("idle_puff", [
        K("rest", 12, REST),
        K("anticipation: a deep breath, head back, eyes shut", 9, P(sy=1.04, head_nod=-6, eye="closed")),
        I("holding it", 6, P(sy=1.05, head_nod=-6, eye="closed", chest=0.03)),
        K("puff: head forward, a smoke ring from the nose", 3,
          P(sy=0.97, head_nod=4, eye="squint", effects=(("puff_big", (17.0, 31.0, 5.0)),))),
        K("the puff drifts up", 6, P(eye="squint", mouth="smirk", effects=(("puff_mid", (19.0, 35.0, 5.0)),))),
        I("and fades", 6, P(eye="squint", effects=(("puff_small", (20.0, 38.0, 5.0)),))),
        H("rest", 15, REST),
    ], once=True, note="An idle variant. Brim blows a smoke puff."),

    Anim("walk", [K(f"{n}", 3, walk_frame(i)) for i, n in enumerate([
        "near foot contact", "down", "passing", "up", "far foot contact", "down", "passing", "up"])],
        flame_step=3, speed=13.0, note="Drawn in place: the simulation moves the Anchor at 13 px/s at 1× (26 px/s at 2×). "
                           "The planted foot moves back at exactly that speed, so it doesn't slide. "
                           "The helmet bobs a drawing behind the body."),

    Anim("run", [K(f"{n}", 2, run_frame(i)) for i, n in enumerate([
        "near foot contact", "push off", "flight", "far foot contact", "push off", "flight"])],
        flame_step=2, speed=35.0, note="For chasing the cursor and running away. Drawn in place at 35 px/s on "
                                       "screen at 1× (70 px/s at 2×), on twos. Leaning in, arms pumping."),

    Anim("sit", [
        K("sitting on the haunches, feet out in front, tail curled behind", 18, P(**SIT)),
        I("breathing in", 6, P(**dict(SIT, chest=0.015))),
        K("full breath", 18, P(**dict(SIT, head_dy=1.0, chest=0.03))),
        I("breathing out", 6, P(**dict(SIT, chest=0.015))),
        K("the tail tip taps", 6, P(**SIT_TAP)),
        K("tap down", 6, P(**SIT)),
        H("rest", 30, P(**SIT)),
    ], note="A lower Body while sitting. The legs and tail rest on the Ground plane, so nothing sinks into it."),

    Anim("sleep", [
        K("dozing off sitting up: hunched forward, head drooping, helmet over the face", 30, P(**SLEEP)),
        I("breathing in", 15, P(**dict(SLEEP, chest=0.025))),
        K("full breath: the back rises", 30, P(**dict(SLEEP, chest=0.05, head_dy=-2.0))),
        I("breathing out", 15, P(**dict(SLEEP, chest=0.025))),
        H("rest", 30, P(**SLEEP)),
    ], flame_step=8, flame_kind="dim", note="Slow breathing. The flame burns low. A lower Body. The 💤 Emote "
                                            "comes from the Emote library."),

    Anim("look", [
        K("turns to look at you: helmet tipped up, eyes round", 30, P(head_turn=20, rock=6, eye="round")),
        K("a little head tilt", 30, P(head_turn=20, head_roll=6, rock=6, eye="round")),
        K("blink", 3, P(head_turn=20, head_roll=6, rock=6, eye="closed")),
        K("back from the tilt", 30, P(head_turn=20, rock=6, eye="round")),
    ], note="Looking at the cursor. Facing left or right is the renderer's mirror."),

    Anim("pounce", [
        K("notices: starts to crouch", 4, P(sy=0.94, head_nod=4, eye="wide")),
        K("anticipation: a deep crouch, rump up", 6,
          P(sy=0.82, sx=1.08, lean=-6, head_nod=6, eye="angry", mouth="flat", near_foot=(0, 0.8), far_foot=(0, 0.8), tail_lift=6)),
        K("rump wiggle left", 3, P(sy=0.82, sx=1.08, lean=-6, head_nod=6, eye="angry", mouth="flat",
                                   near_foot=(0, 0.8), far_foot=(0, 0.8), tail_lift=6, tail_swing=-4)),
        K("rump wiggle right", 3, P(sy=0.82, sx=1.08, lean=-6, head_nod=6, eye="angry", mouth="flat",
                                    near_foot=(0, 0.8), far_foot=(0, 0.8), tail_lift=6, tail_swing=4)),
        K("rump wiggle left", 3, P(sy=0.82, sx=1.08, lean=-6, head_nod=6, eye="angry", mouth="flat",
                                   near_foot=(0, 0.8), far_foot=(0, 0.8), tail_lift=6, tail_swing=-4)),
        S("smear: the launch, stretched along the leap", 2,
          P(lift=6, lean=-16, sx=1.22, sy=0.86, near_foot=(-6, 2), far_foot=(-5, 3), near_hand=(16, 15, 8),
            far_hand=(15, 16, -6), wings=1, eye="wide", mouth="open")),
        K("apex: airborne, claws out", 4,
          P(lift=13, lean=-8, sx=1.06, sy=0.95, near_foot=(-5, 2), far_foot=(-3, 3), near_hand=(16, 17, 8),
            far_hand=(15, 18, -6), wings=2, tail_lift=6, eye="wide", mouth="open")),
        I("coming down", 2, P(lift=7, lean=-14, near_foot=(-2, 1), near_hand=(16, 13, 8), far_hand=(15, 14, -6),
                              wings=1, eye="wide", mouth="open")),
        K("impact: squash, the helmet slams forward", 3,
          P(sy=0.78, sx=1.15, rock=-20, near_hand=(15, 6.5, 8), far_hand=(14, 6.5, -6), near_foot=(0, 0.6),
            eye="closed", mouth="flat")),
        K("rebound: stretch, the helmet bounces back", 3, P(sy=1.06, sx=0.96, rock=8, eye="round", mouth="o")),
        I("settling", 3, P(rock=-2)),
        K("proud: “Ready!”", 12, P(mouth="fang", rock=8, near_hand=(15.0, 13.0, 9.0), tail_lift=5, wings=1)),
        H("rest", 30, REST),
    ], once=True, note="Cursor play (INT-03). The simulation moves the Anchor forward during the launch "
                       "and the flight, about 30 px."),

    Anim("poke", [
        K("hit: squashed sideways, eyes squeezed, the helmet jolts", 2,
          P(sx=1.08, sy=0.9, lean=6, eye="closed", mouth="flat", rock=-10)),
        K("recoil: stretch, the helmet pops up off the head with a “tink”", 3,
          P(sy=1.08, sx=0.95, eye="wide", mouth="o", helmet_lift=4.0, rock=12)),
        I("the helmet comes down", 2, P(eye="wide", mouth="o", helmet_lift=1.0, rock=-6)),
        K("it lands crooked", 3, P(eye="angry", mouth="flat", helmet_roll=10, rock=-4)),
        I("wobbles the other way", 3, P(eye="angry", mouth="flat", helmet_roll=-5)),
        K("settles", 3, P(eye="angry", mouth="flat", helmet_roll=2)),
        H("glare", 24, P(eye="angry", mouth="flat")),
        H("rest", 15, REST),
    ], once=True, note="A single poke (INT-04). Rapid pokes escalate through Emotes and the Mind, not new art."),

    Anim("petted", [
        K("leans into the hand: helmet tipped back, eyes happy, blush; tail wags right", 6,
          P(head_nod=-8, rock=16, eye="happy", mouth="grin", blush=True, tail_swing=4, tail_lift=3)),
        I("wag: the tail passes the middle", 3,
          P(head_nod=-8, head_roll=2, rock=16, eye="happy", mouth="grin", blush=True, tail_swing=0, tail_lift=4)),
        K("nuzzles: head rolls, tail wags left", 6,
          P(head_nod=-8, head_roll=4, rock=16, eye="happy", mouth="grin", blush=True, tail_swing=-4, tail_lift=3)),
        I("wag: the tail passes the middle", 3,
          P(head_nod=-8, head_roll=2, rock=16, eye="happy", mouth="grin", blush=True, tail_swing=0, tail_lift=4)),
    ], flame_step=2, note="Being petted (INT-02). The tail wags and the flame flickers faster."),

    Anim("held", [
        K("dangling: stretched, clutching the helmet", 12,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(0, 1), far_foot=(0, 2), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, rock=-4)),
        I("swing: both feet hang", 4,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(0, 2), far_foot=(0, 2), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, rock=-4)),
        K("kicks the near foot", 8,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(1, 4), far_foot=(-1, 1), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, tail_swing=3, rock=-4)),
        I("swing: both feet hang", 4,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(0, 2), far_foot=(0, 2), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, rock=-4)),
        K("kicks the far foot", 8,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(-1, 1), far_foot=(1, 4), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, tail_swing=-3, rock=-4)),
        I("swing: both feet hang", 4,
          P(lift=5, sy=1.06, sx=0.96, near_foot=(0, 2), far_foot=(0, 2), near_hand=NEAR_BROW, far_hand=FAR_BROW,
            eye="angry", mouth="flat", tail_lift=-8, rock=-4)),
    ], note="The held Situation."),

    Anim("falling", [
        K("panic: wings up, the helmet lifting off", 2,
          P(sy=1.04, wings=2, helmet_lift=3.0, near_hand=NEAR_BROW, far_hand=FAR_BROW, eye="wide", mouth="open",
            tail_lift=8)),
        I("wings level", 2, P(sy=1.04, wings=0.5, helmet_lift=3.5, near_hand=NEAR_BROW, far_hand=FAR_BROW,
                              eye="wide", mouth="open", tail_lift=8)),
        K("wings down", 2, P(sy=1.04, wings=-1, helmet_lift=4.0, near_hand=NEAR_BROW, far_hand=FAR_BROW,
                             eye="wide", mouth="open", tail_lift=8)),
        I("wings level", 2, P(sy=1.04, wings=0.5, helmet_lift=3.5, near_hand=NEAR_BROW, far_hand=FAR_BROW,
                              eye="wide", mouth="open", tail_lift=8)),
    ], flame_step=2, note="The falling Situation, on twos: frantic flapping."),

    Anim("landing", [
        K("impact: a big squash, the helmet clonks over the eyes", 3,
          P(sy=0.72, sx=1.18, rock=-32, near_hand=(13, 6, 9), far_hand=(12, 6, -7), near_foot=(0, 0.6),
            mouth="flat")),
        K("rebound: stretch", 3, P(sy=1.06, sx=0.96, rock=-30, mouth="o")),
        I("settling", 3, P(sy=0.98, rock=-30, mouth="flat")),
        K("dazed: sways under the helmet", 9, P(rock=-30, head_roll=5, mouth="flat")),
        K("pushes the helmet up", 3, P(rock=-6, near_hand=NEAR_BROW, eye="half", mouth="flat")),
        K("shakes it off", 3, P(head_roll=-4, eye="squint")),
        H("rest", 24, REST),
    ], once=True, note="The landing Situation."),

    Anim("hatch", [
        K("the egg, exactly as the egg Form's idle starts", 15, P(egg=dict())),
        K("a wobble left", 4, P(egg=dict(rock=-6, glow=1))),
        K("a wobble right", 4, P(egg=dict(rock=8, glow=1))),
        K("still: the seam glows bright (anticipation)", 12, P(egg=dict(glow=2))),
        K("a big wobble left", 3, P(egg=dict(rock=-10, glow=2))),
        K("a big wobble right", 3, P(egg=dict(rock=12, glow=2))),
        K("squash down: the last anticipation", 4, P(egg=dict(sy=0.9, sx=1.05, glow=2))),
        S("burst: the shell top shoots up, the lower shell cracks into four pieces, Brim stretches up", 2,
          P(helmet_off=True, sy=1.18, sx=0.9, eye="closed", mouth="open", near_hand=(9, 30, 10),
            far_hand=(8, 31, -8), shards=shards(0), shell_top=(-2.0, 50.0, 0.0, -20.0),
            effects=(("spark", (14.0, 22.0, 9.0)), ("spark", (-12.0, 24.0, 9.0))))),
        K("the shell top flies; the pieces fall outward; Brim still stretched, eyes shut", 3,
          P(helmet_off=True, sy=1.1, eye="closed", mouth="open", near_hand=(9, 28, 10), far_hand=(8, 29, -8),
            fx=(("shell_tilt_l", 9, 74), ("shell_tilt_r", 55, 74)), shell_top=(-4.0, 58.0, 0.0, -80.0))),
        K("apex: the shell flips; the pieces lie on the ground; Brim opens its eyes", 4,
          P(helmet_off=True, eye="wide", mouth="o", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)), shell_top=(-6.0, 62.0, 0.0, -160.0))),
        K("blink", 2, P(helmet_off=True, eye="closed", mouth="o", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)),
                        shell_top=(-5.0, 61.0, 0.0, -220.0))),
        K("the shell falls; Brim looks up", 3,
          P(helmet_off=True, eye="round", head_nod=-8, mouth="o", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)),
            shell_top=(-2.0, 52.0, 0.0, -300.0))),
        K("it lands on Brim's head, crooked: squash", 3,
          P(rock=-20, helmet_roll=14, sy=0.9, eye="closed", mouth="flat", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)))),
        K("rebound", 3, P(rock=-16, helmet_roll=-8, sy=1.03, eye="wide", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)))),
        I("the helmet wobbles", 3, P(rock=-16, helmet_roll=4, eye="half", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)))),
        K("puzzled: eyes half shut under the crooked helmet", 12,
          P(rock=-16, helmet_roll=6, eye="half", mouth="flat", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)))),
        K("pushes the helmet straight", 4, P(rock=-2, near_hand=NEAR_BROW, eye="round", fx=(("shell_flat_l", 9, 79), ("shell_flat_r", 54, 79)))),
        K("the pieces crumble into dust", 4,
          P(eye="round", mouth="o", fx=(("puff_big", 9, 79), ("shard_a", 4, 79), ("puff_big", 54, 79),
                                        ("shard_b", 59, 79)))),
        I("the dust settles", 4, P(eye="round", mouth="smirk", fx=(("puff_mid", 9, 79), ("puff_small", 54, 79),
                                                                   ("shard_a", 4, 79)))),
        K("“Ready!”", 30, P(mouth="fang", rock=8, near_hand=(15.0, 13.0, 9.0), tail_lift=5, wings=1,
                            puff=(0, 0))),
        H("rest", 30, REST),
    ], once=True, note="The transformation from egg to hatchling. In format 1 it's an Action of the hatchling Form, "
                       "so item 4 switches Form and then plays it. Its first Frame matches the egg Form's first "
                       "Frame pixel for pixel, so the switch can't be seen."),
]
