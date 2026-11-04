"""Timing sheets: Frames in 30 fps ticks, and turning them into exported Frames.

A timing sheet is a list of F (one drawing each):
  kind   "key", "inbetween", "smear" or "hold"
  label  what happens in it
  ticks  how long it shows, in 30 fps ticks (1 tick = 33.3 ms). On twos = 2, on threes = 3.
  pose   the rig pose; None means "not posed yet", and the animatic holds the previous drawing.

The tail flame flickers on its own clock. An exported Frame never lasts longer than the flame's
step, so a long hold is split into several Frames that differ only in the flame. That's done
here, by a script, so it's the same every time.

Durations in ms come from the running total of ticks, rounded down, so they never drift late:
the Frame changes on exactly the tick the sheet says.
"""

import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "tools"))

import brim  # noqa: E402

TICKS_PER_SECOND = 30
CACHE = os.path.join(ROOT, ".cache")


class F:
    def __init__(self, kind, label, ticks, pose=None, flame_kind=None):
        assert kind in ("key", "inbetween", "smear", "hold"), kind
        assert isinstance(ticks, int) and ticks > 0, ticks
        self.kind, self.label, self.ticks, self.pose = kind, label, ticks, pose
        self.flame_kind = flame_kind


def K(label, ticks, pose=None, **kw):
    return F("key", label, ticks, pose, **kw)


def I(label, ticks, pose=None, **kw):  # noqa: E743
    return F("inbetween", label, ticks, pose, **kw)


def S(label, ticks, pose=None, **kw):
    return F("smear", label, ticks, pose, **kw)


def H(label, ticks, pose=None, **kw):
    return F("hold", label, ticks, pose, **kw)


class Anim:
    """One Animation: its Frames, whether it's meant to play once, and its flame clock."""

    def __init__(self, name, frames, once=False, flame_step=4, flame_kind="normal", note="", form="hatchling",
                 speed=None):
        self.name, self.frames, self.once, self.form = name, frames, once, form
        self.speed = speed  # px/s at 1x that the simulation should move the Anchor, for walk cycles
        self.flame_step, self.flame_kind, self.note = flame_step, flame_kind, note

    @property
    def ticks(self):
        return sum(f.ticks for f in self.frames)


def ms_list(ticks):
    """Per-Frame durations in ms for these tick counts, from the rounded-down running total."""
    out, total, prev = [], 0, 0
    for t in ticks:
        total += t
        now = total * 1000 // TICKS_PER_SECOND
        out.append(now - prev)
        prev = now
    return out


def _rig_version():
    h = hashlib.sha1()
    for name in ("brim.py", "brim3d.py", "pixkit.py"):
        with open(os.path.join(ROOT, "tools", name), "rb") as f:
            h.update(f.read())
    return h.hexdigest()[:12]


RIG_VERSION = _rig_version()


def pose_key(pose):
    return hashlib.sha1((RIG_VERSION + json.dumps(pose, sort_keys=True, default=list)).encode()).hexdigest()[:16]


def base(pose, name="frame", form="hatchling"):
    """The rig's base layer for a pose, cached on disk by the pose, the Form and the rig's source."""
    key = pose_key({"form": form, "pose": pose})
    path = os.path.join(CACHE, key + ".json")
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
        info = data["info"]
        if info.get("flame"):
            info["flame"] = tuple(info["flame"])
        info["effects"] = [tuple(e) for e in info["effects"]]
        return data["grid"], info
    if form == "egg":
        grid, info = brim.draw_egg(name=name, **pose["egg"]), {"flame": None, "effects": []}
    else:
        grid, info = brim.draw_base(brim.pose(**pose) if "lift" not in pose else pose, name)
    os.makedirs(CACHE, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump({"grid": grid, "info": info}, f)
    return grid, info


def drawings(anim):
    """Each drawing of the sheet with the pose it uses: an unposed Frame borrows the last posed one."""
    out, last = [], None
    for i, f in enumerate(anim.frames):
        pose = f.pose if f.pose is not None else last
        if pose is None:
            raise ValueError(f"{anim.name}: Frame {i} has no pose and nothing before it to hold")
        out.append((i, f, pose, f.pose is not None))
        last = pose
    return out


def schedule(anim):
    """Splits the sheet into exported Frames: (drawing index, flame index, ticks)."""
    out, flame = [], 0
    for i, f in enumerate(anim.frames):
        left = f.ticks
        while left > 0:
            step = min(left, anim.flame_step)
            out.append((i, flame, step))
            flame += 1
            left -= step
    return out
