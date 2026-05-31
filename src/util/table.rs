use rustc_hash::{FxBuildHasher, FxHashMap};
use std::num::IntErrorKind;

use crate::util::{IndexOracle, Oracle, all_strs_of_len, construct_str_at, construct_str_idxes_at, k_and_lower};

/// Wrapper around a map of (idx query, response) pairs
pub struct ResponseWrapper {
    map: FxHashMap<usize, bool>,
}

impl ResponseWrapper {
    pub fn from_map(map: FxHashMap<usize, bool>) -> ResponseWrapper {
        ResponseWrapper { map }
    }

    pub fn from_oracle<T: IndexOracle>(oracle: T, sigma: &[char], k: usize) -> ResponseWrapper {
        let mut map = FxHashMap::default();

        for i in 0..k_and_lower(k, sigma.len()) {
            let res = oracle.ask_i(i, sigma);
            map.insert(i, res);
        }

        ResponseWrapper { map }
    }
}

impl Oracle for ResponseWrapper {
    fn ask(&self, _: &str, _: &[char]) -> bool {
        panic!("Use `ask_i` instead of ask.")
    }
}

impl IndexOracle for ResponseWrapper {
    fn ask_i(&self, idx: usize, _: &[char]) -> bool {
        self.map[&idx]
    }
}

/// Wrapper around a map of (string query, response) pairs
pub struct StrResponseWrapper {
    pub size: usize,
    map: FxHashMap<String, bool>,
}

impl StrResponseWrapper {
    pub fn from_map(map: FxHashMap<String, bool>) -> StrResponseWrapper {
        StrResponseWrapper { size: map.len(), map }
    }

    pub fn from_oracle<T: Oracle>(oracle: T, sigma: &[char], k: usize) -> StrResponseWrapper {
        let size = k_and_lower(k, sigma.len());
        let mut map = FxHashMap::with_capacity_and_hasher(size, FxBuildHasher::default());

        for s in all_strs_of_len(sigma, k) {
        // for i in 0..k_and_lower(k, sigma.len()) {
            // let s = construct_str_at(i, sigma);
            let res = oracle.ask(&s, sigma);
            map.insert(s, res);
        }

        StrResponseWrapper { size: map.len(), map }
    }

}

impl Oracle for StrResponseWrapper {
    fn ask(&self, s: &str, _: &[char]) -> bool {
        self.map[s]
    }
}

impl IndexOracle for StrResponseWrapper {
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool {
        let v = construct_str_idxes_at(idx, sigma);
        let s = v.iter().map(|x| sigma[*x]).collect::<String>();
        self.ask(&s, sigma)
    }
}

/// Wrapper around a "table" of answers
pub struct ResponseTable {
    pub size: usize,
    table: Vec<bool>,
}

impl ResponseTable {
    pub fn from_table(table: Vec<bool>) -> ResponseTable {
        ResponseTable {
            size: table.len(),
            table,
        }
    }

    pub fn from_idx_oracle<T: IndexOracle>(oracle: T, sigma: &[char], k: usize) -> ResponseTable {
        let table_size = k_and_lower(k, sigma.len());

        let mut table = Vec::with_capacity(table_size);

        for i in 0..table_size {
            let res = oracle.ask_i(i, sigma);
            table.push(res);
        }

        ResponseTable {
            size: table_size,
            table,
        }
    }
}

impl IndexOracle for ResponseTable {
    fn ask_i(&self, idx: usize, _sigma: &[char]) -> bool {
        self.table[idx]
    }
}

impl Oracle for ResponseTable {
    fn ask(&self, _: &str, _: &[char]) -> bool {
        panic!("Use `ask_i` instead of ask.")
    }
}

/// Function oracle taking an integer as input.
pub struct FnOracle<'a>(pub Box<dyn Fn(usize, usize) -> bool + 'a>);

impl <'a> Oracle for FnOracle<'a> {
    fn ask(&self, input: &str, sigma: &[char]) -> bool {
        let (i, len) = match usize::from_str_radix(input, sigma.len() as u32) {
            Ok(i) => (i, input.len()),
            Err(e) => {
                if let IntErrorKind::Empty = e.kind() {
                    (0,0)
                } else {
                    panic!("Invalid int: {e}")
                }
            }
        };
        self.0(i, len)
    }
}

impl <'a> IndexOracle for FnOracle<'a> {
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool {
        let (i, len) = if idx == 0 {
            (0,0)
        } else {
            // We want to run the oracle on an int representing a string,
            // so we first get the string corresponding to `idx`, then read it as an int.
            let s = construct_str_at(idx, sigma);
            match usize::from_str_radix(&s, sigma.len() as u32) {
                Ok(i) => (i, s.len()),
                Err(e) => panic!("Invalid int: {e}, (str: {})", construct_str_at(idx, sigma)),
            }
        };

        self.0(i, len)
    }
}

