"""The egg Form's Animations: timing sheets with their key poses.

An egg pose is the arguments of brim.draw_egg: rock (degrees, + leans right), sx and sy (squash),
lift, and glow (0 none, 1 faint, 2 bright seam).
"""

from timing import Anim, H, I, K


def E(**kw):
    return {"egg": kw}


ANIMATIONS = [
    Anim("idle", [
        K("still", 60, E()),
        K("a heartbeat: the egg squashes a little", 3, E(sy=0.97, sx=1.02)),
        I("rebound", 3, E(sy=1.01)),
        K("still", 30, E()),
        K("a tiny rock right", 4, E(rock=3)),
        I("back past the middle", 4, E(rock=-1.5)),
        H("still", 30, E()),
    ], note="The default Action, and the egg on first launch (PRG-01). Calm and long, with signs of life."),

    Anim("wobble", [
        K("leans into the hand", 3, E(rock=-8)),
        K("swings right", 4, E(rock=10, sy=0.98)),
        I("", 2, E(rock=4)),
        K("swings left", 3, E(rock=-7)),
        I("", 2, E(rock=-2)),
        K("swings right, smaller", 3, E(rock=4)),
        I("upright, the seam glows faintly", 2, E(glow=1)),
        H("still, glowing", 12, E(glow=1)),
    ], note="Petted or poked (PRG-01). Each swing is smaller: follow-through."),

    Anim("held", [
        K("swings right", 8, E(rock=4)),
        I("", 4, E(rock=1)),
        K("swings left", 8, E(rock=-4)),
        I("", 4, E(rock=-1)),
    ], note="The held Situation: hanging from the cursor like a pendulum."),

    Anim("falling", [
        K("tumbling right, stretched", 2, E(rock=6, sy=1.04, sx=0.97)),
        I("", 2, E(rock=2, sy=1.04, sx=0.97)),
        K("tumbling left", 2, E(rock=-6, sy=1.04, sx=0.97)),
        I("", 2, E(rock=-2, sy=1.04, sx=0.97)),
    ], note="The falling Situation, on twos."),

    Anim("landing", [
        K("impact: squash", 3, E(sy=0.82, sx=1.12)),
        K("rebound: stretch", 3, E(sy=1.06, sx=0.96)),
        K("rocks right", 3, E(rock=5)),
        I("rocks left", 3, E(rock=-3)),
        I("settles", 4, E(rock=1)),
        H("still: the same as idle's first Frame", 30, E()),
    ], once=True, note="The landing Situation."),
]

for _anim in ANIMATIONS:
    _anim.form = "egg"
    _anim.flame_step = 10 ** 6   # the egg has no flame, so long holds stay one Frame
