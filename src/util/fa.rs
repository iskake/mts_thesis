use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use crate::util::{IndexOracle, Oracle, RegExp, UIntN, construct_str_at, construct_str_idxes_at, construct_str_idxes_from};

/********************************
 *
 * Deterministic Finite Automata
 *
 ********************************/

/// Deterministic Finite Automaton
#[derive(Debug)]
pub struct DFA {
    start_state: usize,
    accepts: FxHashSet<usize>,
    transitions: Vec<Vec<usize>>,
    pub n_states: usize,
}

impl DFA {
    /// Manually create a Deterministic Finite Automata using a start state, a set of accept states,
    /// and transitions.
    ///
    /// Use [DFA::from] to create a DFA from a string or regex.
    pub fn new(start_state: usize, accepts: FxHashSet<usize>, transitions: Vec<Vec<usize>>) -> DFA {
        let n_states = transitions.len();
        DFA {
            start_state,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Simulate the DFA on the given input "string" (vec of character indices),
    /// returning `true` if it accepts, and `false` otherwise.
    fn simulate_idx(&self, input: &[usize]) -> bool {
        let mut curr_state = self.start_state;
        for i in input {
            if let Some(h) = self.transitions.get(curr_state)
                && *i != usize::MAX
                && i < &h.len()
                && h[*i] != usize::MAX
            {
                curr_state = self.transitions[curr_state][*i];
            } else {
                return false;
            }
        }
        self.accepts.contains(&curr_state)
    }

    pub fn print_dfa(&self) {
        println!("DFA = {{");
        println!("    \"start_state\": {},", self.start_state);
        println!("    \"accepts\": {:?},", self.accepts);
        println!("    \"transitions\": {:?},", self.transitions);
        println!("    \"n_states\": {},", self.n_states);
        println!("}}");
    }
}

impl From<(&str, &[char])> for DFA {
    fn from(value: (&str, &[char])) -> Self {
        let (r, sigma) = value;
        DFA::from((RegExp::parse(r), sigma))
    }
}

impl From<(RegExp, &[char])> for DFA {
    fn from(rs: (RegExp, &[char])) -> Self {
        let (r, sigma) = rs;
        NDFA::from(r).to_dfa(sigma)
    }
}

impl IndexOracle for DFA {
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool {
        let s = construct_str_idxes_at(idx, sigma);
        self.simulate_idx(&s)
    }
}

impl Oracle for DFA {
    fn ask(&self, s: &str, sigma: &[char]) -> bool {
        let x = construct_str_idxes_from(s, sigma);
        self.simulate_idx(&x)
    }
}



/// Deterministic Finite Automaton (on strings)
#[derive(Debug)]
pub struct StrDFA {
    start_state: usize,
    accepts: FxHashSet<usize>,
    transitions: Vec<FxHashMap<char, usize>>,
    pub n_states: usize,
}

impl StrDFA {
    /// Manually create a Deterministic Finite Automata using a start state, a set of accept states,
    /// and transitions.
    ///
    /// Use [DFA::from] to create a DFA from a string or regex.
    pub fn new(start_state: usize, accepts: FxHashSet<usize>, transitions: Vec<FxHashMap<char, usize>>) -> StrDFA {
        let n_states = transitions.len();
        StrDFA {
            start_state,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Simulate the DFA on the given input "string" (vec of character indices),
    /// returning `true` if it accepts, and `false` otherwise.
    fn simulate_idx(&self, input: &str) -> bool {
        let mut curr_state = self.start_state;
        for c in input.chars() {
            if let Some(h) = self.transitions.get(curr_state)
                && h[&c] != usize::MAX
            {
                curr_state = self.transitions[curr_state][&c];
            } else {
                return false;
            }
        }
        self.accepts.contains(&curr_state)
    }
}

impl From<(DFA, &[char])> for StrDFA {
    fn from(value: (DFA, &[char])) -> Self {
        let (dfa, sigma) = value;
        let transitions = dfa.transitions
            .iter()
            .map(|x| x.iter()
                .enumerate()
                .map(|(i,e)| (sigma[i], *e))
                .collect::<FxHashMap<_,_>>()
            ).collect::<Vec<_>>();
        StrDFA::new(
            dfa.start_state,
            dfa.accepts,
            transitions
        )
    }
}

impl From<(RegExp, &[char])> for StrDFA {
    fn from(rs: (RegExp, &[char])) -> Self {
        let (r, sigma) = rs;
        StrDFA::from((DFA::from((r, sigma)), sigma))
    }
}

impl IndexOracle for StrDFA {
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool {
        let s = construct_str_at(idx, sigma);
        self.simulate_idx(&s)
    }
}

impl Oracle for StrDFA {
    fn ask(&self, s: &str, _: &[char]) -> bool {
        self.simulate_idx(s)
    }
}

/***********************************
 *
 * Nondeterministic Finite Automata
 *
 ***********************************/

/// Non-Deterministic Finite Automaton
#[derive(Debug)]
pub struct NDFA {
    start_state: usize,
    accepts: FxHashSet<usize>,
    transitions: FxHashMap<usize, FxHashMap<char, FxHashSet<usize>>>,
    pub n_states: usize,
}

impl From<&str> for NDFA {
    fn from(value: &str) -> Self {
        NDFA::from(RegExp::parse(value))
    }
}

impl NDFA {
    /// Creates an NDFA from a regular expression.
    ///
    /// Uses the McNaughton-Yamada-Thompson algorithm as described in (Aho, et al. 2006. pp.159-161)
    fn create_ndfa(r: RegExp, i: &mut usize) -> Self {
        use RegExp::*;

        match r {
            Empty => {
                // Since Empty only appears when the input regexp is "", we don't have to
                // care about what `i` is, as there will be only one non-final state.
                NDFA {
                    start_state: 0,
                    accepts: FxHashSet::default(),
                    transitions: FxHashMap::default(),
                    n_states: 1, // ?
                }
            }
            Lambda => NDFA::create_ndfa_lambda(i),
            Symbol(c) => NDFA::create_ndfa_symbol(i, c),
            Plus(r0, r1) => NDFA::create_ndfa_plus(i, r0, r1),
            Concat(r0, r1) => NDFA::create_ndfa_concat(i, r0, r1),
            Star(r) => NDFA::create_ndfa_star(i, r),
        }
    }

    /// Create a NDFA accepting the empty symbol \lambda.
    fn create_ndfa_lambda(i: &mut usize) -> NDFA {
        let q0 = *i;
        *i += 1;

        let f = *i;
        *i += 1;

        let accepts = FxHashSet::from_iter([f]);

        let transitions = FxHashMap::from_iter([(
            q0,
            FxHashMap::from_iter([('\\', FxHashSet::from_iter([f]))]),
        )]);

        let n_states = 2;

        NDFA {
            start_state: q0,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Create a NDFA accepting a single symbol `c`.
    fn create_ndfa_symbol(i: &mut usize, c: char) -> NDFA {
        let q0 = *i;
        *i += 1;

        let f = *i;
        *i += 1;

        let accepts = FxHashSet::from_iter([f]);

        let transitions = FxHashMap::from_iter([(
            q0,
            FxHashMap::from_iter([(c, FxHashSet::from_iter([f]))])
        )]);

        let n_states = 2;

        NDFA {
            start_state: q0,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Create a NDFA from two regular expressions, accepting the disjunction `r0 + r1`
    fn create_ndfa_plus(i: &mut usize, r0: Box<RegExp>, r1: Box<RegExp>) -> NDFA {
        let m0 = NDFA::create_ndfa(r0.as_ref().clone(), i);
        let m1 = NDFA::create_ndfa(r1.as_ref().clone(), i);

        let q0 = *i;
        *i += 1;

        let mut accepts = FxHashSet::default();
        accepts.extend(m0.accepts);
        accepts.extend(m1.accepts);

        let mut transitions = FxHashMap::default();
        transitions.insert(
            q0,
            FxHashMap::from_iter([('\\', FxHashSet::from_iter([m0.start_state, m1.start_state]))]),
        );
        transitions.extend(m0.transitions);
        transitions.extend(m1.transitions);

        let n_states = 1 + m0.n_states + m1.n_states;

        NDFA {
            start_state: q0,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Create a NDFA from two regular expressions, accepting the concatenation `r0 r1`
    fn create_ndfa_concat(i: &mut usize, r0: Box<RegExp>, r1: Box<RegExp>) -> NDFA {
        let mut m0 = NDFA::create_ndfa(r0.as_ref().clone(), i);
        let m1 = NDFA::create_ndfa(r1.as_ref().clone(), i);

        let mut left = VecDeque::from([m0.start_state]);
        let mut found = vec![m0.start_state];

        while !left.is_empty() {
            let q = left.pop_front().unwrap();
            if m0.accepts.contains(&q) {
                if let Some(h) = m0.transitions.get_mut(&q) {
                    if let Some(neighbors) = h.get_mut(&'\\') {
                        neighbors.insert(m1.start_state);
                    } else {
                        h.insert('\\', FxHashSet::from_iter([m1.start_state]));
                    }
                } else {
                    m0.transitions.insert(
                        q,
                        FxHashMap::from_iter([('\\', FxHashSet::from_iter([m1.start_state]))]),
                    );
                }
            }
            if let Some(h) = m0.transitions.get(&q) {
                for neighbors in h.values() {
                    for p in neighbors {
                        if !found.contains(p) {
                            found.push(*p);
                            left.push_back(*p);
                        }
                    }
                }
            }
        }

        m0.accepts = m1.accepts;
        m0.transitions.extend(m1.transitions);

        m0.n_states += m1.n_states;

        m0
    }

    /// Create a NDFA from a regular expression, accepting kleene star `r*`
    fn create_ndfa_star(i: &mut usize, r: Box<RegExp>) -> NDFA {
        let m = NDFA::create_ndfa(r.as_ref().clone(), i);

        let q0 = *i;
        *i += 1;

        let mut accepts = FxHashSet::from_iter([q0]);
        accepts.extend(m.accepts);

        let mut transitions = FxHashMap::from_iter([(
            q0,
            FxHashMap::from_iter([('\\', FxHashSet::from_iter([m.start_state]))]),
        )]);
        transitions.extend(m.transitions);

        let mut left = VecDeque::from([m.start_state]);
        let mut found = vec![m.start_state];

        while !left.is_empty() {
            let q = left.pop_front().unwrap();
            if accepts.contains(&q) {
                if let Some(h) = transitions.get_mut(&q) {
                    if let Some(neighbors) = h.get_mut(&'\\') {
                        neighbors.insert(m.start_state);
                    } else {
                        h.insert('\\', FxHashSet::from_iter([m.start_state]));
                    }
                } else {
                    transitions.insert(
                        q,
                        FxHashMap::from_iter([('\\', FxHashSet::from_iter([m.start_state]))]),
                    );
                }
            }
            for neighbors in transitions[&q].values() {
                for p in neighbors {
                    if !found.contains(p) {
                        found.push(*p);
                        left.push_back(*p);
                    }
                }
            }
        }

        let n_states = 1 + m.n_states;

        NDFA {
            start_state: q0,
            accepts,
            transitions,
            n_states,
        }
    }

    /// Get the set of states reachable from set `s` using a lambda-step.
    ///
    /// This is a helper function for simulating the NDFA. See (Aho, et al. 2006. p.153)
    /// (Note: referred to as epsilon closure in the book)
    fn lambda_closure(&self, t: FxHashSet<usize>) -> FxHashSet<usize> {
        let mut stack = VecDeque::from(t.iter().copied().collect::<Vec<_>>());
        let mut e_c = t;

        while !stack.is_empty() {
            let e = stack.pop_back().unwrap();
            if self.transitions.contains_key(&e)
                && let Some(us) = self.transitions[&e].get(&'\\')
            {
                for u in us {
                    if !e_c.contains(u) {
                        e_c.insert(*u);
                        stack.push_back(*u);
                    }
                }
            }
        }
        e_c
    }

    /// Get the set of states reachable from set `s` using `c`.
    ///
    /// This is a helper function for simulating the NDFA. See (Aho, et al. 2006. p.153)
    fn move_(&self, s: &FxHashSet<usize>, c: char) -> FxHashSet<usize> {
        let mut s_ = FxHashSet::default();
        for e in s {
            if let Some(h) = self.transitions.get(e)
                && h.contains_key(&c)
            {
                s_.extend(self.transitions[e].get(&c).unwrap());
            }
        }
        s_
    }

    /// Simulate the NDFA, as described in (Aho, et al. 2006. p.156)
    fn simulate(&self, input: &str) -> bool {
        let mut s = self.lambda_closure(FxHashSet::from_iter([self.start_state]));

        for c in input.chars() {
            s = self.lambda_closure(self.move_(&s, c));
        }

        s.iter().any(|e| self.accepts.contains(e))
    }
}

impl From<RegExp> for NDFA {
    fn from(value: RegExp) -> Self {
        let mut i = 0usize;
        NDFA::create_ndfa(value, &mut i)
    }
}

impl NDFA {
    /// Conversion from NDFA to DFA, as described in (Aho, et al. 2006. p.153)
    fn to_dfa(self, sigma: &[char]) -> DFA {
        let mut i = 0;
        let start_state = i;
        i += 1;
        let mut accepts = FxHashSet::default();
        let mut transitions: Vec<Vec<usize>> = vec![];

        let mut dstates = FxHashMap::default();

        let mut q0 = UIntN::new(self.n_states);
        let dstate_set = self.lambda_closure(FxHashSet::from_iter([self.start_state]));
        dstate_set.iter().for_each(|x| q0.set_bit(*x));

        dstates.insert(q0.clone(), start_state);

        let mut unmarked = VecDeque::new();
        unmarked.push_back(q0);

        while !unmarked.is_empty() {
            let t = unmarked.pop_front().unwrap();
            let t_id = dstates[&t];

            let t_set = &t.get_set_bits();

            if t_set.iter().any(|x| self.accepts.contains(x)) {
                accepts.insert(t_id);
            }

            // "mark t"

            for (char_id, c) in sigma.iter().enumerate() {
                let u_set = self.lambda_closure(self.move_(t_set, *c));
                let mut u = UIntN::new(self.n_states);
                u_set.iter().for_each(|x| u.set_bit(*x));
                let u_id;

                if !dstates.contains_key(&u) {
                    // println!("adding {u} to dstates");
                    unmarked.push_back(u.clone());

                    u_id = i;
                    i += 1;

                    dstates.insert(u, u_id);
                } else {
                    // println!("{u} already in dstates!");
                    u_id = dstates[&u];
                }

                // Add transition t --c-> u
                if let Some(h) = transitions.get_mut(t_id) {
                    h[char_id] = u_id;
                } else {
                    let mut v = vec![usize::MAX; sigma.len()];
                    v[char_id] = u_id;
                    transitions.insert(t_id, v);
                    // transitions.insert(t_id, FxHashMap::from_iter([(*c, u_id)]));
                }
            }
        }

        DFA {
            start_state,
            accepts,
            transitions,
            n_states: i,
        }
    }
}

impl Oracle for NDFA {
    fn ask(&self, input: &str, _: &[char]) -> bool {
        self.simulate(input)
    }
}

impl IndexOracle for NDFA {
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool {
        self.simulate(&construct_str_at(idx, sigma))
    }
}

/******************************
 *
 *          Testing
 *
 ******************************/
#[cfg(test)]
mod test {
    use crate::util::*;
    use fa::*;

    #[test]
    fn test_fa() {
        let mut machine_sextuples: Vec<(
            &str,
            &[char],
            &DFA,
            &NDFA,
            &DFA,
            &dyn Fn(&String) -> bool,
        )> = Vec::new();

        let regex = "(0+1)*00";
        let sigma = ['0', '1', '2'];
        let fa = DFA::new(
            0,
            FxHashSet::from_iter([2]),
            vec![
                vec![1, 0, usize::MAX],
                vec![2, 0, usize::MAX],
                vec![2, 0, usize::MAX],
            ],
        );
        let ndfa = NDFA::from(regex);
        let dfa = DFA::from((regex, sigma.as_slice()));
        let f = |s: &String| !s.contains('2') && s.ends_with("00");
        machine_sextuples.push((regex, &sigma, &fa, &ndfa, &dfa, &f));

        // ab* for Sigma = {a,b,c}
        let regex = "ab*";
        let sigma = ['a', 'b', 'c'];
        let fa = DFA::new(
            0,
            FxHashSet::from_iter([1]),
            vec![
                vec![1, usize::MAX, usize::MAX],
                vec![usize::MAX, 1, usize::MAX],
            ],
        );
        let ndfa = NDFA::from(regex);
        let dfa = DFA::from((regex, sigma.as_slice()));
        let f = |s: &String| {
            let mut should_acc = false;
            let mut seen_a = false;
            for c in s.chars() {
                if c == 'a' && !seen_a {
                    seen_a = true;
                    should_acc = true;
                } else if c == 'b' && !seen_a {
                    should_acc = false;
                    break;
                } else if c == 'b' && seen_a {
                    continue;
                } else if c == 'a' && seen_a {
                    should_acc = false;
                    break;
                } else {
                    should_acc = false;
                    break;
                }
            }
            should_acc
        };
        machine_sextuples.push((regex, &sigma, &fa, &ndfa, &dfa, &f));

        // a*b*c for Sigma = {a,b,c}
        let regex = "a*b*c";
        let sigma = ['a', 'b', 'c'];
        let fa = DFA::new(
            0,
            FxHashSet::from_iter([2]),
            vec![
                vec![0, 1, 2],
                vec![usize::MAX, 1, 2],
                vec![usize::MAX, usize::MAX, usize::MAX],
            ],
        );
        let ndfa = NDFA::from(regex);
        let dfa = DFA::from((regex, sigma.as_slice()));
        let f = |s: &String| {
            let mut seen_b = false;
            let mut seen_c = false;
            for c in s.chars() {
                if c == 'a' && !seen_b && !seen_c {
                    continue;
                } else if c == 'b' && !seen_c {
                    seen_b = true;
                } else if c == 'c' && !seen_c {
                    seen_c = true;
                } else {
                    return false;
                }
            }
            seen_c
        };
        machine_sextuples.push((regex, &sigma, &fa, &ndfa, &dfa, &f));


        // {} for Sigma = {a,b,c} with 10 (empty) states
        let regex = "";
        let fa = DFA::new(
            9,
            FxHashSet::default(),
            vec![vec![]; 10],
        );
        let ndfa = NDFA::from(regex);
        let dfa = DFA::from((regex, sigma.as_slice()));
        let f = |_: &String| false;
        machine_sextuples.push((regex, &sigma, &fa, &ndfa, &dfa, &f));

        // λ for Sigma = {a,b,c}
        let regex = "\\";
        let fa = DFA::new(
            0,
            FxHashSet::from_iter([0]),
            vec![vec![]],
        );
        let ndfa = NDFA::from(regex);
        let dfa = DFA::from((regex, sigma.as_slice()));
        let f = |s: &String| s.is_empty();
        machine_sextuples.push((regex, &sigma, &fa, &ndfa, &dfa, &f));

        for (regex, sigma, fa, ndfa, dfa, tm) in machine_sextuples {
            for i in 0..k_and_lower(8, sigma.len()) {
                // Run TM (function) with the same language as the FAs
                let should_acc = tm(&construct_str_at(i, sigma));

                // Actually run it
                if should_acc {
                    assert!(fa.ask_i(i, sigma),   "manual fa  rejected {} when it should have accepted! (regex: {regex})", construct_str_at(i, sigma));
                    assert!(dfa.ask_i(i, sigma),  "regex dfa  rejected {} when it should have accepted! (regex: {regex})", construct_str_at(i, sigma));
                    assert!(ndfa.ask_i(i, sigma), "regex ndfa rejected {} when it should have accepted! (regex: {regex})", construct_str_at(i, sigma));
                } else {
                    assert!(!fa.ask_i(i, sigma),   "manual fa   accepted {} when it should have rejected! (regex: {regex})", construct_str_at(i, sigma));
                    assert!(!dfa.ask_i(i, sigma),  "regex dfa   accepted {} when it should have rejected! (regex: {regex})", construct_str_at(i, sigma));
                    assert!(!ndfa.ask_i(i, sigma), "regex ndfa  accepted {} when it should have rejected! (regex: {regex})", construct_str_at(i, sigma));
                }
            }
        }
    }

    #[test]
    fn test_equiv_ndfa() {
        let mut thing: Vec<(&[char], NDFA, NDFA)> = Vec::new();
        thing.push((
            &['0', '1'],
            NDFA::from("(000)+(100)"),
            NDFA::from("(0+1)00"),
        ));
        thing.push((
            &['0', '1', '2'],
            NDFA::from("(000)+(100)"),
            NDFA::from("(0+1)00"),
        ));

        thing.push((
            &['a', 'b', 'c'],
            NDFA::from("(a*b*c*)*"),
            NDFA::from("((a+b)+c)*"),
        ));
        thing.push((&['a', 'b'], NDFA::from("(a*b*)*"), NDFA::from("(b*a*)*")));

        for (sigma, m0, m1) in thing {
            for i in 0..k_and_lower(7, sigma.len()) {
                assert_eq!(
                    m0.ask_i(i, sigma),
                    m1.ask_i(i, sigma),
                    "for string {}",
                    construct_str_at(i, sigma)
                );
            }
        }
    }

    #[test]
    fn test_dfa_ndfa_from_regex_str() {
        let sigma_regex_pairs: Vec<(&[char], &str)> = vec![
            (&['0', '1'], "(0+1)*01"),
            (&['0', '1'], "(0+(01))*01010"),
            (&['a', 'b', 'c'], "a*b*c*"),
        ];

        for (sigma, regex) in sigma_regex_pairs {
            let ndfa = NDFA::from(regex);
            let dfa = DFA::from((regex, sigma));

            for i in 0..k_and_lower(10, sigma.len()) {
                assert_eq!(
                    dfa.ask_i(i, sigma),
                    ndfa.ask_i(i, sigma),
                    "for string {}",
                    construct_str_at(i, sigma)
                );
            }
        }
    }
}
