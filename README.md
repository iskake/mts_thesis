# MTS implementation

This repository includes the implementation of the Method of Test Sets algortihms described in (Landøy, T. 2026).

## Structure

The repository is structured in the following way:
- `src/` contains the MTS implementations and two programs for: benchmarking each algorithm (ch.4), and discovering the structure of minimal DFAs (ch.5), respectively.
- `util/` contains utilities for visualization.
  - `plot.py` creates plots from the data obtained using `bench`
  - `dfa_vis.py` gives a rough visualization of what each DFA looks like

## Requirements

In order to run any of the Rust code, you will need Cargo and a Rust compiler.  
Note: all code has been tested purely on *nix systems (macOS and Linux.)
No support for any other opterating system (such as Windows) is guaranteed or implied.

- Rust code (see [`Cargo.toml`](Cargo.toml))
  - rust version 1.94.0
  - `rustc-hash` version 2.1.1
- Python code (util)
  - `plot.py` - matplotlib, numpy
  - `dfa_vis.py` - matplotlib, pillow, automathon
  - Install with: `pip install matplotlib numpy pillow automathon`
    - (automathon additionally requires [graphviz](https://graphviz.org/download/) to be installed)
    - Install with: `sudo apt install graphviz` (debian-based linux) or `brew install graphviz` (brew on macOS) ...or some other way.

## Running

Each program can be run in the following way:

### `bench`

```bash
# Run benchmarks on language 1 with alphabet of size 2,
# and output the resulting (latex) table to the "table/" directory.
~ $ cargo run --release --bin bench l_1 2 "table/"
# Afterwards, plot the values and save the results in "figures/"
~ $ python3 util/plot.py all 100 50 2 "table/table_((0+1))*0^k.tex" "figures/" "lang_1"

# To use a different alphabet size, simply change the parameter:
~ $ cargo run --release --bin bench 3
```

### `discover`

```bash
# Find minimal automaton for L(n) with n = 2, |\Sigma| = 2
~ $ cargo run --release --bin discover l_n 2 2
[...]
Constructed dfa:
DFA = {
    "start_state": 0,
    "accepts": {7, 5, 6, 4},
    "transitions": [[0, 1], [2, 3], [4, 5], [6, 7], [0, 1], [2, 3], [4, 5], [6, 7]],
    "n_states": 8,
}
[...]

# Then, after copying the above DFA into `dfa_vis.py`, visualize (roughly) 
# what the automaton looks like:
~ $ python3 util/dfa_vis.py
```

## License

This repository is dual-licensed under the [MIT License](https://spdx.org/licenses/MIT.html) (see [LICENSE-MIT](LICENSE-MIT)) and [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/) (see [LICENSE-CC-BY-4.0](LICENSE-CC-BY-4.0)).

<!-- This is because the thesis itself is licensed under CC-BY-4.0 and includes a zip of this repository. So, we can't really include this repository there without also having the same license. -->
