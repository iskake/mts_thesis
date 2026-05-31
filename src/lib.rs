// LISTING mts START
pub mod util;

use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
};

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use crate::util::{
    IndexOracle, Oracle, UIntN, all_strs_of_len, concat_idxes, construct_str_at, fa::DFA,
    k_and_lower,
};

#[derive(Debug)]
pub struct BrokenPromiseError;

/// Push onto string `s` using either
/// - if `all_strs` is `Some` - the string at index `i` into `all_strs`, or
/// - otherwise - the string constructed directly from index `i` using `sigma`.
macro_rules! push_either {
    ($s:expr, $i:expr, $sigma:expr, $all_strs:expr) => {
        match $all_strs {
            Some(ref strs) => $s.push_str(&strs[$i]),
            None => $s.push_str(&construct_str_at($i, $sigma)),
        }
    };
}

/// Get the `i`-th string via either
/// - if `all_strs` is `Some` - the string at index `i` into `all_strs`, or
/// - otherwise - the string constructed directly from index `i` using `sigma`.
macro_rules! get_either {
    ($i:expr, $sigma:expr, $all_strs:expr) => {
        match $all_strs {
            Some(ref strs) => &strs[$i],
            None => &construct_str_at($i, $sigma),
        }
    };
}

/// Options to the 'optimized' method of test sets algorithms.
/// Chooses which specific optimizations and variations to use.
pub enum MTSOpt {
    None,        // 'Pre-speedup'
    Index,       // 'All-speedup'
    String,      // 'String'
    StringIndex, // 'String index'
}

/********************************
 *                              *
 *       PRE-SPEEDUP MTS        *
 *         ALGORITHMS           *
 *                              *
 ********************************/

/********************************
 *
 *     Pre-speedup Naive MTS
 *
 ********************************/

/// Naive implementation of the Method of Test Sets (table generation),
/// using the methods described in (Sloper, C. 2001. pp.43-46) and
/// (Downey, R.G. Fellows, M.R. 1999).
///
/// Note that the original algorithms (by Sloper / Downey & Fellows, etc.)
/// do not actually _create_ a DFA, only the test set table, then using
/// Myhill-Nerode say one can construct a DFA from this.
fn mts_create_table<T: IndexOracle + Oracle>(
    sigma: &[char],
    k: usize,
    oracle: &T,
    strs: &[String],
) -> Vec<Vec<bool>> {
    let r = k_and_lower(k - 1, sigma.len());
    let c = k_and_lower(k - 2, sigma.len());
    let mut t = vec![vec![false; c]; r];

    for i in 0..r {
        let s_i = &strs[i];
        for j in 0..c {
            let s_j = &strs[j];

            let mut s = String::from(s_i);
            s.push_str(s_j);

            t[i][j] = oracle.ask(&s, sigma)
        }
    }

    t
}

/// Function for creating a DFA, using the test set table obtained from
/// [mts_create_table].
pub fn mts_naive_unopt<T: IndexOracle + Oracle>(
    sigma: &[char],
    k: usize,
    oracle: &T,
    _: MTSOpt,
) -> Result<DFA, BrokenPromiseError> {
    let c = k_and_lower(k - 2, sigma.len());
    let strs = all_strs_of_len(sigma, k - 1);

    let t = mts_create_table(sigma, k, oracle, &strs);

    let mut eq = FxHashSet::default();
    let mut s_queue = VecDeque::new();
    let mut r_map = FxHashMap::default(); // row -> state
    let mut q_c = 0;

    for (i, e) in t.iter().enumerate() {
        if !eq.contains(e) {
            let s_i = &strs[i];
            s_queue.push_back(s_i);
            r_map.insert(e, q_c);
            eq.insert(e);
            q_c += 1;
        }
    }

    let mut accepts = FxHashSet::default();
    let mut transitions = vec![vec![usize::MAX; sigma.len()]; q_c];
    q_c = 0;

    for s_i in s_queue {
        for (i_z, z) in sigma.iter().enumerate() {
            let mut a = vec![false; c];
            for j in 0..c {
                let s_j = &strs[j];

                let mut s = String::from(s_i);
                s.push(*z);
                s.push_str(&s_j);

                a[j] = oracle.ask(&s, sigma)
            }

            let p = r_map[&a];
            if a[0] {
                accepts.insert(p);
            }
            transitions[q_c][i_z] = p;
        }
        q_c += 1;
    }

    let m = DFA::new(0, accepts, transitions);

    Ok(m)
}

/***********************************************
 *
 *  Pre-speedup On-the-fly Method of Test Sets
 *
 ***********************************************/
#[derive(Default)]
enum UnoptTreePart {
    Branch(Box<UnoptTreePart>, usize, Box<UnoptTreePart>),
    Leaf(usize),
    #[default]
    Nothing,
}
type UnoptTree = Box<UnoptTreePart>;

// Signature test. Mutates the signature tree, searching for leaf values. (unopt. alg.)
fn signature_test_unopt<T: IndexOracle + Oracle>(
    s: &String,
    t: &mut UnoptTree,
    oracle: &T,
    sigma: &[char],
    tests_len: usize,
    max_id: &mut usize,
    all_strs: &[String],
    curr_idx: usize,
) -> Result<usize, BrokenPromiseError> {
    match t.as_mut() {
        UnoptTreePart::Branch(l, _, r) => {
            // Branch, so go down to the right or left depending on whether the string is
            // part of the language or not.
            let mut s_ = String::from(s);
            s_.push_str(&all_strs[curr_idx]);
            let res = oracle.ask(&s_, sigma);

            // Go either down node `lr` in the tree.
            // If `lr` doesn't exist (i.e. it is "Nothing",) add a new branch or leaf.
            macro_rules! go_lr {
                ($lr:ident) => {{
                    if let UnoptTreePart::Nothing = **$lr {
                        // println!("    creating node!");
                        if curr_idx >= tests_len - 1 {
                            *$lr = Box::new(UnoptTreePart::Leaf(*max_id));
                            *max_id += 1;
                        } else {
                            *$lr = Box::new(UnoptTreePart::Branch(
                                Box::new(UnoptTreePart::Nothing),
                                curr_idx + 1,
                                Box::new(UnoptTreePart::Nothing),
                            ));
                        }
                    }
                    signature_test_unopt(
                        s,
                        $lr,
                        oracle,
                        sigma,
                        tests_len,
                        max_id,
                        all_strs,
                        curr_idx + 1,
                    )
                }};
            }

            if res { go_lr!(r) } else { go_lr!(l) }
        }
        UnoptTreePart::Leaf(x) => Ok(*x),
        UnoptTreePart::Nothing => unreachable!(),
    }
}

// Fast signature test. Does not mutate the tree. (unopt. alg.)
fn signature_test_fast_unopt<T: IndexOracle + Oracle>(
    s: &String,
    t: &UnoptTree,
    oracle: &T,
    sigma: &[char],
    all_strs: &[String],
    curr_idx: usize,
) -> usize {
    match t.as_ref() {
        UnoptTreePart::Branch(l, curr_str, r) => {
            let mut test_str = String::from(s);
            test_str.push_str(&all_strs[*curr_str]);
            let res = oracle.ask(&test_str, sigma);

            let b = if res { r } else { l };

            signature_test_fast_unopt(s, b, oracle, sigma, all_strs, curr_idx)
        }
        UnoptTreePart::Leaf(x) => *x,
        UnoptTreePart::Nothing => unreachable!(),
    }
}

/// Contract tree together (unopt. alg.)
fn contract_unopt(t: &mut UnoptTree) {
    match t.as_mut() {
        &mut UnoptTreePart::Branch(ref mut l, _, ref mut r) => {
            contract_unopt(l);
            contract_unopt(r);

            let l_old = std::mem::take(l);
            let r_old = std::mem::take(r);

            match (*l_old, *r_old) {
                (UnoptTreePart::Nothing, UnoptTreePart::Nothing) => {
                    unreachable!()
                }
                (l_, UnoptTreePart::Nothing) => {
                    *t = Box::new(l_);
                }
                (UnoptTreePart::Nothing, r_) => {
                    *t = Box::new(r_);
                }
                (l_, r_) => {
                    *l = Box::new(l_);
                    *r = Box::new(r_);
                }
            }
        }
        _ => {}
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct State {
    signature_id: usize,
    equivalence_string: String,
}

/// 'On the fly' implementation of the Method of Test Sets, as described
/// in (Sloper, C. 2001. pp.47-58)
pub fn mts_otf_unopt<T: IndexOracle + Oracle>(
    sigma: &[char],
    k: usize,
    oracle: &T,
    _: MTSOpt,
) -> Result<DFA, BrokenPromiseError> {
    let all_strs = all_strs_of_len(sigma, k);
    let all_strs_len = k_and_lower(k - 2, sigma.len());
    let mut max_node_id = 0;
    let mut tree = Box::new(UnoptTreePart::Branch(
        Box::new(UnoptTreePart::Nothing),
        0,
        Box::new(UnoptTreePart::Nothing),
    ));
    let mut transitions: Vec<Vec<usize>> = vec![vec![usize::MAX; sigma.len()]; k];

    let mut q_set = FxHashSet::default();
    let mut k_queue = VecDeque::new();

    let str = String::from("");
    let signature_id = signature_test_unopt(
        &str,
        &mut tree,
        oracle,
        sigma,
        all_strs_len,
        &mut max_node_id,
        &all_strs,
        0,
    )?;
    let q = State {
        signature_id,
        equivalence_string: String::from(""),
    };

    q_set.insert(q.signature_id);
    k_queue.push_back(q);

    let mut contracted = false;

    while let Some(q) = k_queue.pop_front() {
        // println!("  q: {q:?}");
        for (a_idx, a) in sigma.iter().enumerate() {
            if !contracted && q_set.len() == k {
                contract_unopt(&mut tree);
                contracted = true;
            }

            let mut str = String::from(&q.equivalence_string);
            str.push(*a);

            let s = if !contracted {
                signature_test_unopt(
                    &str,
                    &mut tree,
                    oracle,
                    sigma,
                    all_strs_len,
                    &mut max_node_id,
                    &all_strs,
                    0,
                )?
            } else {
                signature_test_fast_unopt(&str, &tree, oracle, sigma, &all_strs, 0)
            };

            if let Some(t) = q_set.get(&s) {
                // println!(" sig_id: {}, a_idx: {}, t: {}", q.signature_id, a_idx, t);
                transitions[q.signature_id][a_idx] = *t;
            } else {
                let signature_id = s;
                let mut equivalence_string = String::from(&q.equivalence_string);
                equivalence_string.push(*a);
                let v = State {
                    signature_id,
                    equivalence_string,
                };

                transitions[q.signature_id][a_idx] = v.signature_id;

                q_set.insert(v.signature_id);
                k_queue.push_back(v);
            }
        }
    }

    let start_state = 0;
    let accepts = match *tree {
        UnoptTreePart::Branch(_, _, r) => leaf_values_unopt(r),
        _ => FxHashSet::default(),
    };

    Ok(DFA::new(start_state, accepts, transitions))
}

/// Traverse the tree downwards and find all leaf nodes (unopt. tree)
fn leaf_values_unopt(tree: UnoptTree) -> FxHashSet<usize> {
    let mut values = FxHashSet::default();
    let mut to_check = VecDeque::from([tree]);

    'l: while let Some(elem) = to_check.pop_front() {
        match *elem {
            UnoptTreePart::Branch(l, _, r) => {
                to_check.push_back(l);
                to_check.push_back(r);
            }
            UnoptTreePart::Leaf(id) => {
                if id == usize::MAX {
                    break 'l;
                }
                values.insert(id);
            }
            UnoptTreePart::Nothing => {}
        }
    }
    values
}

/***********************************
 *                                 *
 *  OPTIMIZED (all-speedup, etc.)  *
 *         MTS ALGORITHMS          *
 *                                 *
 ***********************************/

/********************************
 *
 *  Naive Method of Test Sets
 *
 ********************************/

/// Naive implementation of the Method of Test Sets.
///
/// Based on (Sloper, C. 2001. pp.43-46)
pub fn mts_naive<T: IndexOracle + Oracle>(
    sigma: &[char],
    k: usize,
    oracle: &T,
    optimize: MTSOpt,
) -> Result<DFA, BrokenPromiseError> {
    // Optimize / 'unoptimize' the test set table
    let rows = k_and_lower(k - 1, sigma.len());
    let cols = k_and_lower(k - 2, sigma.len());

    // Vec and hashmap to store `row -> signature`, and `signature -> row`, respectively
    let mut equiv_class_of_row = Vec::with_capacity(rows);
    let mut equiv_classes = FxHashMap::default();
    let mut equiv_classes_arr = vec![usize::MAX; k];

    let all_strs = match optimize {
        // Create set of all strings <= k - 1 if using the string algorithm.
        MTSOpt::String => Some(all_strs_of_len(sigma, k - 1)),
        _ => None,
    };

    // The set of "real" states, i.e. the states the actual DFA will contain
    // starting from state 0 and going up
    let mut curr_state = 0;
    // ? optimization: use `vec![usize::MAX; rows]` to improve speed at cost of memory?
    let mut row_to_state = FxHashMap::default();

    // The set of accepting states
    let mut accepts = FxHashSet::default();

    // Create the signature for each row in the table, then
    for i in 0..rows {
        let signature = match optimize {
            MTSOpt::Index => get_signature(i, 0, oracle, sigma, cols),
            _ => {
                let x = get_either!(i, sigma, all_strs);
                get_signature_str(x, None, oracle, sigma, cols, all_strs.as_deref())
            }
        };
        let signature_hash = hash_single(&signature);

        // Store this signature if it has not appeared for any other rows yet
        // (i.e., we found a "new" equivalence class)
        if !equiv_classes.contains_key(&signature_hash) {
            equiv_classes.insert(signature_hash, i);
            equiv_classes_arr[curr_state] = i;

            // First bit (0) is set, i.e. first test set answer is true, when the
            // string is in the language, so this equivalence class will be one of
            // the accepting states.
            if signature.is_bit_set(0) {
                accepts.insert(curr_state);
            }

            // We "create a new state" to represent this equivalence class
            row_to_state.insert(i, curr_state);
            curr_state += 1;
        }
        equiv_class_of_row.push(signature_hash);
    }

    // Done creating the 'table', start construction of the DFA

    // Start state is always 0 i.e. [\lambda]
    let start_state = 0;
    let n_states = curr_state;
    let mut transitions = vec![vec![usize::MAX; sigma.len()]; n_states];

    // Create the transitions of the DFA
    for (state_idx, q) in equiv_classes_arr.iter().enumerate() {
        if *q == usize::MAX {
            // Found less than k equivalence classes, no reason to continue iterating.
            break;
        }

        let neighbors = transitions.get_mut(state_idx).unwrap();
        for i in 0..sigma.len() {
            // Get the index of the row of the table corresponding to the
            // eq class [qz] (z \in \Sigma)
            // e.g. for Sigma = { 0, 1 } and q = 1 ('0')
            //      when z = '0' (qz = '00'), idx = 3:
            // |idx|str|
            // | 0 |\  |
            // | 1 |0  |
            // | 2 |1  |
            // |(3)|00 |
            // | 4 |01 |
            // |...|...|
            let row_idx = (sigma.len() - 1) * (*q) + *q + i + 1;

            if row_idx < rows {
                // Index already computed so we can just get it
                let eq_c_of_idx = equiv_class_of_row[row_idx];
                let eq_c_row = equiv_classes[&eq_c_of_idx];
                // The row of the corresponding equivalence class
                let row = eq_c_row;

                neighbors[i] = row_to_state[&row];
            } else {
                // Table has too few rows for index! we have to compute it now!
                let signature = match optimize {
                    MTSOpt::Index => get_signature(*q, i + 1, oracle, sigma, cols),
                    _ => {
                        let x = get_either!(*q, sigma, all_strs);
                        get_signature_str(
                            x,
                            Some(sigma[i]),
                            oracle,
                            sigma,
                            cols,
                            all_strs.as_deref(),
                        )
                    }
                };

                let signature_hash = hash_single(&signature);
                let row = equiv_classes[&signature_hash];

                neighbors[i] = row_to_state[&row];
            }
        }
    }

    Ok(DFA::new(start_state, accepts, transitions))
}

/// Get the signature of the equivalence class `[xa]` where `x` is a string
/// and `a` is a single symbol.
fn get_signature<T: IndexOracle>(
    x: usize,
    a: usize,
    oracle: &T,
    sigma: &[char],
    len: usize,
) -> UIntN {
    let mut pattern = UIntN::new(len);

    for j in 0..len {
        let idx0 = concat_idxes(x, a, sigma.len());
        let idx1 = concat_idxes(idx0, j, sigma.len());

        let accept = oracle.ask_i(idx1, sigma);
        if accept {
            pattern.set_bit(j);
        }
    }

    pattern
}

/// Get the signature of the equivalence class `[xa]` where `x` is a string
/// and `a` is a single symbol.
fn get_signature_str<T: Oracle>(
    x: &str,
    a: Option<char>,
    oracle: &T,
    sigma: &[char],
    len: usize,
    all_strs: Option<&[String]>,
) -> UIntN {
    let mut pattern = UIntN::new(len);

    for j in 0..len {
        let mut s = String::from(x);
        if let Some(c) = a {
            s.push(c);
        }
        push_either!(s, j, sigma, all_strs);

        let accept = oracle.ask(&s, sigma);
        if accept {
            pattern.set_bit(j);
        }
    }

    pattern
}

/// Hash a single value.
fn hash_single<H: Hash>(pattern: &H) -> u64 {
    let mut hasher = FxHasher::default();
    pattern.hash(&mut hasher);
    hasher.finish()
}

/***********************************
 *
 *  On-the-fly Method of Test Sets
 *
 ***********************************/

enum Either<L, R> {
    Left(L),
    Right(R),
}

/// Part of a tree, either a branch or a leaf
#[derive(Clone, Copy, Debug)]
enum TreePart {
    /// Branch containing (idx to left branch, string idx, idx to right branch)
    Branch(usize, usize, usize),
    /// Leaf. Holds a single `usize` (in the context of MTS, this is a state)
    Leaf(usize),
}

type Tree = [TreePart];

/// Signature test. Mutates the signature tree, searching for leaf values.
fn signature_test<T: IndexOracle + Oracle>(
    string_idx: &Either<String, usize>,
    t: &mut Tree,
    oracle: &T,
    sigma: &[char],
    tests_len: usize,
    max_id: &mut usize,
    alloc_ptr: &mut usize,
    all_strs: Option<&[String]>,
) -> Result<usize, BrokenPromiseError> {
    let mut curr_tree_idx = 0;

    for i in 0..tests_len {
        let res = match string_idx {
            Either::Left(s) => {
                let mut s_ = String::from(s);
                push_either!(s_, i, sigma, all_strs);
                oracle.ask(&s_, sigma)
            }
            Either::Right(s_idx) => {
                let idx = concat_idxes(*s_idx, i, sigma.len());
                oracle.ask_i(idx, sigma)
            }
        };

        match t[curr_tree_idx] {
            TreePart::Branch(l, _, r) => {
                let mut subtree_idx = if res { r } else { l };

                // i.e., no child defined
                if subtree_idx == usize::MAX {
                    let to_write = if i >= tests_len - 1 {
                        let l = TreePart::Leaf(*max_id);
                        *max_id += 1;
                        l
                    } else {
                        TreePart::Branch(usize::MAX, i + 1, usize::MAX)
                    };
                    subtree_idx = *alloc_ptr;
                    *alloc_ptr += 1;

                    t[subtree_idx] = to_write;

                    if let TreePart::Branch(l, j, r) = t[curr_tree_idx] {
                        let new_l = if res { l } else { subtree_idx };
                        let new_r = if res { subtree_idx } else { r };
                        t[curr_tree_idx] = TreePart::Branch(new_l, j, new_r)
                    }
                }
                curr_tree_idx = subtree_idx;

                if let TreePart::Leaf(id) = t[curr_tree_idx] {
                    return Ok(id);
                }
            }
            _ => unreachable!(),
        }
    }
    unreachable!();
}

/// Faster signature test. Does not mutate the signature tree, it only traverses down.
fn signature_test_fast<T: IndexOracle + Oracle>(
    string_idx: &Either<String, usize>,
    t: &mut Tree,
    oracle: &T,
    sigma: &[char],
    all_strs: Option<&[String]>,
) -> usize {
    let mut curr_idx = 0;

    while let TreePart::Branch(l, i, r) = t[curr_idx] {
        let res = match string_idx {
            Either::Left(s) => {
                let mut s_ = String::from(s);

                push_either!(s_, i, sigma, all_strs);
                oracle.ask(&s_, sigma)
            }
            Either::Right(s_idx) => {
                let s_idx_idx = concat_idxes(*s_idx, i, sigma.len());
                oracle.ask_i(s_idx_idx, sigma)
            }
        };

        curr_idx = if res { r } else { l };
    }

    if let TreePart::Leaf(x) = t[curr_idx] {
        x
    } else {
        unreachable!()
    }
}

/// Contract tree together
fn contract(t: &mut Tree) {
    let start_idx = 0;
    let mut to_check = VecDeque::from([start_idx]);

    while let Some(elem) = to_check.pop_front() {
        match t[elem] {
            TreePart::Branch(usize::MAX, _, usize::MAX) => {
                unreachable!()
            }
            TreePart::Branch(l, _, usize::MAX) => {
                t[elem] = t[l];
                to_check.push_back(elem);
            }
            TreePart::Branch(usize::MAX, _, r) => {
                t[elem] = t[r];
                to_check.push_back(elem);
            }
            TreePart::Branch(l, _, r) => {
                to_check.push_back(l);
                to_check.push_back(r);
            }
            TreePart::Leaf(_) => {}
        }
    }
}

/// Traverse the tree downwards and find all leaf nodes.
fn leaf_values(tree: &[TreePart], start_idx: usize) -> FxHashSet<usize> {
    let mut values = FxHashSet::default();
    let mut to_check = VecDeque::from([start_idx]);

    'l: while let Some(elem) = to_check.pop_front() {
        if elem == usize::MAX {
            break 'l;
        }

        match tree[elem] {
            TreePart::Branch(l, _, r) => {
                if l != usize::MAX {
                    to_check.push_back(l);
                }
                if r != usize::MAX {
                    to_check.push_back(r);
                }
            }
            TreePart::Leaf(id) => {
                values.insert(id);
            }
        }
    }
    values
}

/// 'On the fly' implementation of the Method of Test Sets.
///
/// Based on (Sloper, C. 2001. pp.47-58)
pub fn mts_otf<T: IndexOracle + Oracle>(
    sigma: &[char],
    k: usize,
    oracle: &T,
    optimize: MTSOpt,
) -> Result<DFA, BrokenPromiseError> {
    // Amount of strings of length at most k - 2
    let all_strs_len = k_and_lower(k - 2, sigma.len());

    let all_strs = match optimize {
        MTSOpt::String => Some(all_strs_of_len(sigma, k)),
        _ => None,
    };

    // Create the "tree"
    // Note that the tree has size `1 + k * [tree height] + k`, where:
    // - `1` - for root node
    // - `k * [tree heigth]` - max size of all paths. See (Sloper, C. 2001. p.49)
    // - `k` - the leaf nodes, of which there are k total.
    let mut tree = vec![TreePart::Branch(usize::MAX, 0, usize::MAX); 1 + k * all_strs_len + k];
    let mut max_node_id = 0;
    let mut contracted = false;

    let mut transitions: Vec<Vec<usize>> = vec![vec![usize::MAX; sigma.len()]; k];
    let mut q_set = FxHashSet::default();
    let mut k_queue = VecDeque::new();

    // Add the first node
    let q0 = 0usize;
    let str_idx = if let MTSOpt::Index = optimize {
        Either::Right(0)
    } else {
        Either::Left(String::from(""))
    };
    let mut tree_alloc_ptr = 1;

    let signature = signature_test(
        &str_idx,
        &mut tree,
        oracle,
        sigma,
        all_strs_len,
        &mut max_node_id,
        &mut tree_alloc_ptr,
        all_strs.as_deref()
    )?;

    k_queue.push_back((q0, str_idx));
    q_set.insert(signature);

    while !k_queue.is_empty() {
        let (q, str_idx) = k_queue.pop_front().unwrap();

        for (i, a) in sigma.iter().enumerate() {
            // If we have already found k signatures (states), we can contract the tree.
            if q_set.len() >= k && !contracted {
                contract(&mut tree);
                contracted = true;
            }

            let concat_str_idx = match str_idx {
                Either::Left(ref s) => {
                    let mut str = String::from(s);
                    str.push(*a);
                    Either::Left(str)
                }
                Either::Right(s_idx) => Either::Right(concat_idxes(s_idx, 1 + i, sigma.len())),
            };

            let signature = if !contracted {
                signature_test(
                    &concat_str_idx,
                    &mut tree,
                    oracle,
                    sigma,
                    all_strs_len,
                    &mut max_node_id,
                    &mut tree_alloc_ptr,
                    all_strs.as_deref()
                )?
            } else {
                signature_test_fast(
                    &concat_str_idx,
                    &mut tree,
                    oracle,
                    sigma,
                    all_strs.as_deref()
                )
            };

            if let Some(t) = q_set.get(&signature) {
                // Signature already exists for t, add transition from q --i-> t
                transitions[q][i] = *t;
            } else {
                // Found new signature! Add new state v, and add transition q --i--> v
                let v = signature;
                transitions[q][i] = v;

                q_set.insert(v);
                k_queue.push_back((v, concat_str_idx));
            }
        }
    }

    let start_state = 0;
    let accepts = match tree[0] {
        TreePart::Branch(_, _, usize::MAX) => FxHashSet::default(),
        TreePart::Branch(_, _, r) => leaf_values(&tree, r),
        _ => unreachable!(),
    };

    Ok(DFA::new(start_state, accepts, transitions))
}

// LISTING mts END
/******************************
 *
 *          Testing
 *
 ******************************/

#[cfg(test)]
mod test {
    use crate::{
        util::{IndexOracle, all_strs_of_len},
        *,
    };

    fn test_mts(test_fn: fn(&[char], usize, &DFA, MTSOpt) -> Result<DFA, BrokenPromiseError>) {
        let mut test_cases: Vec<(_, &[char], _, _, _, _)> = Vec::new();

        let regex = "((000)+1)*(0+1)*";
        let sigma = ['0', '1'];
        let k = 6;
        let ndfa = DFA::from((regex, sigma.as_slice()));
        test_cases.push((regex, &sigma, k, ndfa, MTSOpt::Index, 8));

        let regex = "(0+1)*00";
        let sigma = ['0', '1'];
        let k = 3;
        let ndfa = DFA::from((regex, sigma.as_slice()));
        test_cases.push((regex, &sigma, k, ndfa, MTSOpt::Index, 8));

        let regex = "(0+1)*011";
        let sigma = ['0', '1'];
        let k = 4;
        let ndfa = DFA::from((regex, sigma.as_slice()));
        test_cases.push((regex, &sigma, k, ndfa, MTSOpt::Index, 8));

        let regex = "a*b*c";
        let sigma = ['a', 'b', 'c'];
        let k = 4;
        let ndfa = DFA::from((regex, sigma.as_slice()));
        test_cases.push((regex, &sigma, k, ndfa, MTSOpt::Index, 8));

        let regex = "101010";
        let sigma = ['0', '1'];
        let k = 8;
        let ndfa = DFA::from((regex, sigma.as_slice()));
        test_cases.push((regex, &sigma, k, ndfa, MTSOpt::Index, 10));

        for (regex, sigma, k, fa, opt, str_lens) in test_cases {
            println!("testing for mts regex: {regex}");
            let m_dfa = test_fn(&sigma, k, &fa, opt).unwrap();

            let strs = all_strs_of_len(&sigma, str_lens);
            for (i, _) in (&strs).iter().enumerate() {
                assert_eq!(
                    m_dfa.ask_i(i, sigma),
                    fa.ask_i(i, sigma),
                    "failed for '{regex}', mts fa: {m_dfa:#?}"
                );
            }
            assert!(
                m_dfa.n_states <= k,
                "expected mts dfa to have {k} states, has {}",
                m_dfa.n_states
            );
            assert!(
                m_dfa.n_states <= fa.n_states,
                "mts dfa: {}, normal dfa: {}, {m_dfa:#?}",
                m_dfa.n_states,
                fa.n_states
            );
        }
    }

    #[test]
    fn test_naive() {
        test_mts(mts_naive::<DFA>);
    }

    #[test]
    fn test_otf() {
        test_mts(mts_otf::<DFA>);
    }

    #[test]
    fn test_naive_unopt() {
        test_mts(mts_naive_unopt::<DFA>);
    }

    #[test]
    fn test_otf_unopt() {
        test_mts(mts_otf_unopt::<DFA>);
    }
}
