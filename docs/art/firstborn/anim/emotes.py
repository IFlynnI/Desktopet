"""The Emote library: one per motive the Mind can show (REN-04). Cell 16 x 16, drawn by hand.

Each Emote is a list of Frames: (label, ticks, [(shape, x, y), ...]). A shape is a small hand-drawn
grid of palette keys ('.' is empty), placed with its top-left at (x, y) in the 16 x 16 cell. Every
Frame then gets the style guide's 1 px outline in `o` round its shapes, so the shapes are drawn
without one and must stay 1 px clear of the cell edge.
"""

import pixkit as px

CELL = (16, 16)

# ---------------------------------------------------------------- shapes

Z_SMALL = ["vvvv",
           "..v.",
           ".v..",
           "vvvv"]
Z_BIG = ["vvvvv",
         "...vu",
         "..vu.",
         ".vu..",
         "vvvvv"]
Z_FADE = ["uuuu",
          "..u.",
          ".u..",
          "uuuu"]

HEART_SMALL = ["rr.rr",
               "rrrrr",
               ".rrr.",
               "..r.."]
HEART = [".rr.rr.",
         "rprrrrr",
         "rrrrrrr",
         ".rrrrr.",
         "..rrr..",
         "...r..."]
HEART_SQUASH = [".rr..rr.",
                "rprrrrrr",
                "rrrrrrrr",
                ".rrrrrr.",
                "...rr..."]
HEART_TALL = [".rr.rr.",
              "rprrrrr",
              "rprrrrr",
              "rrrrrrr",
              ".rrrrr.",
              "..rrr..",
              "...r..."]
HEART_TINY = ["r.r",
              "rrr",
              ".r."]

BANG_TALL = ["yy",
             "Yy",
             "yy",
             "yy",
             "yy",
             "yr",
             "..",
             "yy",
             "yr"]
BANG_SQUASH = ["yYyy",
               "yyyr",
               ".yr.",
               "....",
               ".yr."]
BANG = ["yYy",
        "yyy",
        "yyr",
        ".yr",
        "...",
        ".yr"]
BANG_SMALL = ["yy",
              "yr",
              "..",
              "yr"]

QUESTION = [".4444.",
            "45..44",
            "....44",
            "...44.",
            "..44..",
            "......",
            "..44.."]
QUESTION_TILT = ["..4444",
                 ".45..4",
                 ".....4",
                 "...44.",
                 "..44..",
                 "......",
                 ".44..."]

VEIN = [".rr...rr.",
        "rrr...rrr",
        "rr.....rr",
        ".........",
        ".........",
        ".........",
        "rr.....rr",
        "rrr...rrr",
        ".rr...rr."]
VEIN_SMALL = ["rr...rr",
              "r.....r",
              ".......",
              ".......",
              ".......",
              "r.....r",
              "rr...rr"]
VEIN_MID = [".r...r.",
            "rr...rr",
            ".......",
            ".......",
            ".......",
            "rr...rr",
            ".r...r."]

# ---------------------------------------------------------------- the library

EMOTES = [
    ("tired", "looping", [
        ("a small z rises", 10, [(Z_SMALL, 3, 9)]),
        ("a bigger Z", 10, [(Z_BIG, 5, 5)]),
        ("the Z drifts, a new z", 10, [(Z_BIG, 8, 2), (Z_SMALL, 2, 10)]),
        ("the Z fades", 10, [(Z_FADE, 10, 1), (Z_SMALL, 3, 7)]),
    ]),
    ("love", "one-shot", [
        ("a small heart pops in", 2, [(HEART_SMALL, 6, 7)]),
        ("big: squash", 2, [(HEART_SQUASH, 4, 6)]),
        ("overshoot", 3, [(HEART_TALL, 5, 4)]),
        ("settles", 12, [(HEART, 5, 5)]),
        ("floats up", 6, [(HEART, 5, 3)]),
        ("shrinks away", 4, [(HEART_TINY, 7, 2)]),
    ]),
    ("notice", "one-shot", [
        ("❗ pops in, stretched tall", 2, [(BANG_TALL, 7, 3)]),
        ("squash", 2, [(BANG_SQUASH, 6, 6)]),
        ("settles", 15, [(BANG, 6, 4)]),
        ("shrinks", 3, [(BANG_SMALL, 7, 7)]),
    ]),
    ("curious", "looping", [
        ("❓ up", 8, [(QUESTION, 5, 3)]),
        ("middle, tilting", 8, [(QUESTION_TILT, 5, 4)]),
        ("down", 8, [(QUESTION, 5, 5)]),
        ("middle, tilting back", 8, [(QUESTION, 4, 4)]),
    ]),
    ("annoyed", "looping", [
        ("💢 small", 6, [(VEIN_SMALL, 4, 4)]),
        ("big", 4, [(VEIN, 3, 3)]),
        ("middle", 6, [(VEIN_MID, 4, 4)]),
    ]),
]


def outline(grid, key="o"):
    """Adds a 1 px outline in `key` round every drawn pixel (4-neighbour)."""
    h, w = len(grid), len(grid[0])
    rows = [list(r) for r in grid]
    for y in range(h):
        for x in range(w):
            if grid[y][x] != px.TRANSPARENT:
                continue
            if any(0 <= x + dx < w and 0 <= y + dy < h and grid[y + dy][x + dx] != px.TRANSPARENT
                   for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                rows[y][x] = key
    return ["".join(r) for r in rows]


def frame(shapes, name="emote"):
    grid = px.blank(*CELL)
    for shape, x, y in shapes:
        if x < 1 or y < 1 or x + len(shape[0]) > CELL[0] - 1 or y + len(shape) > CELL[1] - 1:
            raise ValueError(f"{name}: a shape at ({x}, {y}) leaves no room for its outline inside the cell")
        grid = px.paste(grid, shape, x, y)
    return outline(grid)


def frames(emote):
    name, mode, fr = emote
    return [frame(shapes, f"{name}[{i}]") for i, (_, _, shapes) in enumerate(fr)]
