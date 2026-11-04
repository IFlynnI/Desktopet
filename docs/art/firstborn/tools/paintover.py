"""The paint-over layer: hand corrections laid on top of the rig's base layer.

A paint-over file is a pixel-grid file (see pixkit.write_grid_file) whose frames use:
  '.'  keep the base pixel
  '_'  erase the base pixel (make it transparent)
  any palette key: paint that colour

Each paint-over frame records the hash of the base frame it was painted on (`base=<hash>` in its
header). If the rig changes and the base no longer matches, compose() refuses, naming the frame, so
a correction never lands on the wrong pixels without anyone noticing.

Usage, from export.py:
  start(path, frames, palette)   writes an empty paint-over for every frame that has none
  apply(path, frames)            returns the frames with the paint-over laid on
"""

import hashlib
import os

import pixkit as px

ERASE = "_"


def base_hash(grid):
    return hashlib.sha1("\n".join(grid).encode("utf-8")).hexdigest()[:10]


def read(path):
    """Returns {frame name: (hash, grid)}; an absent file means no corrections."""
    if not os.path.exists(path):
        return {}
    out = {}
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    name, want, h, rows = None, None, None, []

    def flush():
        if name is not None:
            if len(rows) != want[1]:
                raise ValueError(f"{path}: paint-over frame {name} has {len(rows)} rows, wants {want[1]}")
            out[name] = (h, rows[:])

    for n, line in enumerate(lines, 1):
        if line.startswith("frame "):
            flush()
            parts = line.split()
            name = parts[1]
            want = tuple(int(v) for v in parts[2].split("x"))
            h = dict(t.split("=", 1) for t in parts[3:]).get("base")
            rows = []
        elif name is not None and not line.startswith("#"):
            if len(line) != want[0]:
                raise ValueError(f"{path}:{n}: row is {len(line)} wide, frame {name} is {want[0]}")
            rows.append(line)
    flush()
    return out


def write(path, frames, corrections, note=""):
    """frames: [(name, base grid)]; corrections: {name: grid} (missing means empty)."""
    lines = ["# firstborn paint-over, format 1",
             "# '.' keeps the base pixel, '_' erases it, a palette key paints it.",
             "# base=<hash> is the base Frame this was painted on; export refuses a mismatch."]
    if note:
        lines += [f"# {n}" for n in note.splitlines()]
    for name, grid in frames:
        w, h = px.grid_size(grid)
        lines.append(f"frame {name} {w}x{h} base={base_hash(grid)}")
        lines += corrections.get(name) or [px.TRANSPARENT * w for _ in range(h)]
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def is_empty(grid):
    return all(set(r) <= {px.TRANSPARENT} for r in grid)


def apply(path, frames):
    """Lays the paint-over on the base frames. Raises if a non-empty correction's base has changed."""
    found = read(path)
    out, stale = [], []
    for name, grid in frames:
        if name not in found:
            out.append((name, grid))
            continue
        h, fix = found[name]
        if is_empty(fix):
            out.append((name, grid))
            continue
        if h != base_hash(grid):
            stale.append(name)
            out.append((name, grid))
            continue
        rows = [list(r) for r in grid]
        for y, line in enumerate(fix):
            for x, ch in enumerate(line):
                if ch == ERASE:
                    rows[y][x] = px.TRANSPARENT
                elif ch != px.TRANSPARENT:
                    rows[y][x] = ch
        out.append((name, ["".join(r) for r in rows]))
    if stale:
        raise ValueError(f"{path}: the base changed under the paint-over for frame(s) {', '.join(stale)}. "
                         "Redo those corrections, or restore the rig.")
    return out


def start(path, frames, note=""):
    """Writes a paint-over with a Frame for every base Frame. Existing corrections are kept, with the
    fingerprint they were painted on, so a stale one is still caught. An empty Frame has nothing to go
    stale, so it takes the current base's fingerprint."""
    found = read(path)
    lines = ["# firstborn paint-over, format 1",
             "# '.' keeps the base pixel, '_' erases it, a palette key paints it.",
             "# base=<hash> is the base Frame this was painted on; export refuses a mismatch."]
    if note:
        lines += [f"# {n}" for n in note.splitlines()]
    for name, grid in frames:
        w, h = px.grid_size(grid)
        old = found.get(name)
        if old and not is_empty(old[1]):
            lines.append(f"frame {name} {w}x{h} base={old[0]}")
            lines += old[1]
        else:
            lines.append(f"frame {name} {w}x{h} base={base_hash(grid)}")
            lines += [px.TRANSPARENT * w for _ in range(h)]
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def edits(grid, changes):
    """Builds a correction grid from {(x, y): key} the same size as `grid`."""
    w, h = px.grid_size(grid)
    rows = [[px.TRANSPARENT] * w for _ in range(h)]
    for (x, y), k in changes.items():
        rows[y][x] = k
    return ["".join(r) for r in rows]


def write_raw(path, entries, note=""):
    """Writes a sparse paint-over: entries are (name, width, height, base hash, rows), corrected
    Frames only. Removes the file if there are none."""
    entries = [e for e in entries if not is_empty(e[4])]
    if not entries:
        if os.path.exists(path):
            os.remove(path)
        return
    lines = ["# firstborn paint-over, format 1",
             "# '.' keeps the base pixel, '_' erases it, a palette key paints it.",
             "# base=<hash> is the base Frame this was painted on; export refuses a mismatch.",
             "# Only corrected Frames are stored. Edit with tools/paint.py, or by hand."]
    if note:
        lines += [f"# {n}" for n in note.splitlines()]
    for name, w, h, digest, rows in sorted(entries):
        lines.append(f"frame {name} {w}x{h} base={digest}")
        lines += rows
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def prune(path):
    """Drops the empty Frames from a paint-over file."""
    found = read(path)
    write_raw(path, [(n, len(g[0]), len(g), h, g) for n, (h, g) in found.items()])
