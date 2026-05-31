# pyright: basic

import sys
import matplotlib.pyplot as plt
import numpy as np

if len(sys.argv) != 8 or sys.argv[1] not in ["all", "otf"]:
    print("usage: python3 [all|otf] const_naive const_otf sigma_size plot.py path/to/table/file.tex out_dir out_file_suffix")
    print("  e.g. python3 all plot.py 100 50 2 \"../latex/figures/table_(0+1)*0^k.tex\" \"../latex/figures/\" \"(0+1)*^k\"")
    exit(0)

table_entries = sys.argv[1]
const_n = int(sys.argv[2])
const_o = int(sys.argv[3])
sigma_size = int(sys.argv[4])
table_path = sys.argv[5]
outfile_dir = sys.argv[6]
outfile_suffix = sys.argv[7]

with open(table_path, "r") as f:
    lines = map(lambda x: x.replace("~", ""), f.readlines())

table = []
start_k = 999

k_lines = []

for line in lines:
    x = line.split("&")
    k_lines.append((x[0], x[1]))
    k = int(\
        x[1].replace("$", "")\
            .replace("k=", "")\
            .replace("(","")\
            .replace(")",""))

    start_k = min(start_k, k)

    data = np.array([int(z.replace("\\\\", "").strip()) for z in x[2:]])
    # print(k, data)
    table.append(data)
table = np.array(table).T
r,c = table.shape


mts_types = [
    "Naive",
    "On-the-fly "
]

line_names = np.array([
    "Pre-speedup",
    "String",
    "String Index",
    "All-speedup",
])

if table_entries == "all":
    mts_line_names = [f"{typ} ({name.lower()})" for typ in mts_types for name in line_names]
else:
    mts_line_names = [f"On-the-fly ({name.lower()})" for name in line_names]

plt.rcParams.update({
    "text.usetex": True,
    "font.family": "Helvetica"
})


# 
# All
# 

fig, ax = plt.subplots(ncols=1, nrows=1, figsize=(4, 4), layout="constrained")
# plt.subplots_adjust(hspace=0.35)

# const_n = 100
# const_o = 50
y_n_fn = lambda x: [sigma_size**(2 * e - 3) * const_n for e in x]
y_o_fn = lambda x: [e * (sigma_size**(e - 1)) * const_o for e in x]
y_n_lab = "$|\\Sigma|^{2k - 3} \\cdot " + str(const_n) + "$"
y_o_lab = "$k \\cdot |\\Sigma|^{k - 1} \\cdot " + str(const_o) + "$"

for i, y in enumerate(table):
    x = range(start_k, start_k + len(y))
    ax.plot(x, y, label=mts_line_names[i])

ax.set_xlabel("k")
ax.set_ylabel("Nanoseconds")
ax.set_title(f"Naive and on-the-fly (log scale)")
ax.set_yscale('log')
# else:
    # ax.ticklabel_format(useOffset=True, useMathText=True)


x = range(start_k, start_k + c)
if table_entries == "all":
    ax.plot(x, y_n_fn(x), label=y_n_lab, color='#ff00ff', linestyle="dashed")
    ax.plot(x, y_o_fn(x), label=y_o_lab, color='#000000', linestyle="dashed")

# else:
ax.grid(True)
ax.legend(prop={'size': 7})

filename = outfile_dir + "plot_" + outfile_suffix + f"_{table_entries}.png"
fig.savefig(filename, dpi=300)
print(f"Saved file {filename}")


if table_entries == "all":
    # 
    # Individual
    # 
    nrows = 2 if table_entries == "all" else 1

    fig, axs = plt.subplots(ncols=1, nrows=nrows, figsize=(4, 8), layout="constrained")
    # plt.subplots_adjust(hspace=0.35, wspace=0.25)

    for row, ax in enumerate(axs):
        if table_entries == "all":
            start = (r // 2) * row
            end   = r // (2 - row)
        else:
            start = 0
            end = r

        # print(start, end, table[start:end])
        for i, y in enumerate(table[start:end]):
            x = range(start_k, start_k + c)
            ax.plot(x, y, label=line_names[i])

        x = range(start_k, start_k + c)
        if table_entries == "all" and row == 0:
            ax.plot(x, y_n_fn(x), label=y_n_lab, color="#ff00ff", linestyle="dashed")
        else:
            ax.plot(x, y_o_fn(x), label=y_o_lab, color="#000000", linestyle="dashed")

        ax.set_title(f"{mts_types[row]} (log scale)")
        ax.set_xlabel("k")
        ax.set_ylabel("Nanoseconds")
        ax.set_yscale('log')
        ax.grid(True)
        ax.legend()

    filename = outfile_dir + "plot_" + outfile_suffix + "_naive_otf.png"
    fig.savefig(filename, dpi=300)
    print(f"Saved file {filename}")

    #
    # Only the pre-speedup ones
    #
    const_n = 300
    const_o = 100
    y_n_fn = lambda x: [2**(2 * e - 3) * const_n for e in x]
    y_o_fn = lambda x: [e * (2**(e - 1)) * const_o for e in x]
    y_n_lab = "$|\\Sigma|^{2k - 3} \\cdot " + str(const_n) + "$"
    y_o_lab = "$k \\cdot |\\Sigma|^{k - 1} \\cdot " + str(const_o) + "$"
    
    no_opt_idxes = [
        3,
        7,
    ]
    
    fig, ax = plt.subplots(ncols=1, nrows=1, figsize=(4, 4), layout="constrained")

    for i in range(0,2):
        idx = no_opt_idxes[i]
        x = range(start_k, start_k + c)
        y = table[idx]
        ax.plot(x, y, label=mts_line_names[idx])

    ax.set_xlabel("k")
    ax.set_ylabel("Nanoseconds")
    ax.set_title(f"Naive and on-the-fly (log scale)")
    ax.set_yscale('log')

    x = range(start_k, start_k + c)
    ax.plot(x, y_n_fn(x), label=y_n_lab, color='#ff00ff', linestyle="dashed")
    ax.plot(x, y_o_fn(x), label=y_o_lab, color='#000000', linestyle="dashed")

    # else:
    ax.grid(True)
    ax.legend(prop={'size': 7})
    
    
    filename = outfile_dir + "plot_" + outfile_suffix + "_no_opt.png"
    fig.savefig(filename, dpi=300)
    print(f"Saved file {filename}")
    
    s = ""
    for i in range(c):
        s += f"{k_lines[i][0]}&{k_lines[i][1]}& {table[no_opt_idxes[0], i]:_} & {table[no_opt_idxes[1], i]:_}\\\\\n"
    s = s.replace("_", "~")
    
    filename = outfile_dir + "table_" + outfile_suffix + "_no_opt.tex"
    with open(filename, "w") as f:
        f.write(s)
    print(f"Saved file {filename}")


    #
    # Graph showing just the speedup
    #
    all_opt_idxes = [
        3,
        7,
    ]

    no_opt_idxes = [
        0,
        4,
    ]
    
    fig, ax = plt.subplots(ncols=1, nrows=1, figsize=(4, 4), layout="constrained")

    print(table[all_opt_idxes[0]])
    print(table[no_opt_idxes[0]])
    print(table[no_opt_idxes[0]] / table[all_opt_idxes[0]])
    for i in range(0,2):
        all_idx = all_opt_idxes[i]
        no_idx = no_opt_idxes[i]
        x = range(start_k, start_k + c)
        y = table[no_idx] / table[all_idx]

        coef = np.polyfit(x, y, 1)
        poly1d_fn = np.poly1d(coef)

        colors = ["tab:blue", "tab:orange"]

        ax.plot(x, y, 'o', color=colors[i], label=mts_types[i] + " speedup")
        ax.plot(x, poly1d_fn(x), '--', color=colors[i], label=mts_types[i] + " spd.up (regression)")

    ax.set_xlabel("k")
    ax.set_ylabel("Relative speedup (all-speedup / pre-speedup)")
    ax.set_title(f"Naive and on-the-fly (speedup)")

    x = range(start_k, start_k + c)

    # else:
    ax.grid(True)
    ax.legend(prop={'size': 7})
    
    filename = outfile_dir + "plot_" + outfile_suffix + "_speedup.png"
    fig.savefig(filename, dpi=150)
    print(f"Saved file {filename}")
    
    # s = ""
    # for i in range(c):
    #     s += f"{k_lines[i][0]}&{k_lines[i][1]}& {table[no_opt_idxes[0], i]:_} & {table[no_opt_idxes[1], i]:_}\\\\\n"
    # s = s.replace("_", "~")
    
    # filename = outfile_dir + "table_" + outfile_suffix + "_no_opt.tex"
    # with open(filename, "w") as f:
    #     f.write(s)
    # print(f"Saved file {filename}")

# plt.show()
