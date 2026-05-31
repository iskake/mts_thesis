# Little python script to visualize a DFA obtained from
# the `DFA::print_dfa` function (defined in `src/util/fa.rs`)

# pyright: basic

# Replace the following DFA with values obtained from `discover`

# vvvvvvvv REPLACE vvvvvvvvv
DFA = {
    "start_state": 0,
    "accepts": {11, 3, 14, 6, 17, 9, 12, 15, 4, 18, 7, 10, 13, 16, 19, 8},
    "transitions": [[0, 1], [0, 2], [0, 3], [4, 5], [6, 7], [0, 5], [8, 9], [6, 10], [0, 11], [8, 12], [6, 13], [0, 14], [8, 15], [4, 16], [0, 17], [4, 18], [6, 16], [4, 19], [8, 18], [0, 19]],
    "n_states": 20,
}
# ^^^^^^^^ REPLACE ^^^^^^^^^

SIGMA = [str(x) for x in range(0, 10)]

import automathon

q = { *map(str, range(0, DFA["n_states"]))}
sigma = { *SIGMA }
delta = { str(p): {SIGMA[i]: str(q) for i,q in enumerate(trans)} for p,trans in enumerate(DFA["transitions"])}
initial_state = str(0)
f = { *map(str, DFA["accepts"]) }

dfa = automathon.DFA(q, sigma, delta, initial_state, f)
print("is valid dfa?:", dfa.is_valid())
dfa.view("dfa")


from PIL import Image
import matplotlib.pyplot as plt
import os

img = Image.open("dfa.gv.png")

os.remove("dfa.gv")
os.remove("dfa.gv.png")

plt.imshow(img)
plt.show()

