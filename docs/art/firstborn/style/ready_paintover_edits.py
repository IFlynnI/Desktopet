# Writes the paint-over file for the signature pose with its hand corrections.
import sys
sys.path.insert(0, "tools"); sys.path.insert(0, "style")
import brim as R, paintover as po, poses

base = R.draw(poses.READY)
EDITS = {}
# helmet highlight: a short arc down the dome instead of a block (the mouth is drawn by the rig now)
for xy in [(31, 34), (32, 34), (31, 35), (33, 35), (34, 35), (32, 36), (33, 36)]:
    EDITS[xy] = "v"
if len(sys.argv) > 1 and sys.argv[1] == "dump":
    print("    " + "".join(str(i // 10) for i in range(64)))
    print("    " + "".join(str(i % 10) for i in range(64)))
    for y, r in enumerate(base):
        print(f"{y:3d} {r}")
    sys.exit()
po.write("style/ready.paintover.txt", [("ready", base)], {"ready": po.edits(base, EDITS)},
         note="Hand corrections for the signature pose on the gate 2 style guide.")
print("ok")
