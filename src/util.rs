pub mod fa;
pub mod table;

use std::{fmt, hash::Hash, iter::Peekable, str::Chars};

/// Regular expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegExp {
    Empty,
    Lambda,
    Symbol(char),
    Plus(Box<RegExp>, Box<RegExp>),
    Concat(Box<RegExp>, Box<RegExp>),
    Star(Box<RegExp>),
}

use RegExp::*;
use rustc_hash::{FxHashMap, FxHashSet};

impl fmt::Display for RegExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Empty => write!(f, "{{}}"),
            Lambda => write!(f, "λ"),
            Symbol(c) => write!(f, "{c}"),
            Concat(r, s) => write!(f, "{r}{s}"),
            Plus(r, s) => match (*r.clone(), *s.clone()) {
                (Concat(..), Concat(..)) => write!(f, "({r})+({s})"),
                (Concat(..), _) => write!(f, "(({r})+{s})"),
                (_, Concat(..)) => write!(f, "({r}+({s}))"),
                _ => write!(f, "({r}+{s})"),
            },
            Star(r) => match **r {
                Symbol(_) | Plus(..) => write!(f, "{r}*"),
                _ => write!(f, "({r})*"),
            },
        }
    }
}

impl RegExp {
    /// Parse a regular expression from a string.
    pub fn parse(s: &str) -> RegExp {
        let mut chars = s.chars().peekable();

        if chars.peek().is_none() {
            Empty
        } else {
            let mut r = RegExp::parse_regexp_concat(&mut chars);
            while chars.peek().is_some() {
                // ?should be concat or done somewhere else?
                r = Concat(
                    Box::new(r),
                    Box::new(RegExp::parse_regexp_concat(&mut chars)),
                );
            }
            r
        }
    }

    fn parse_regexp_concat(chars: &mut Peekable<Chars<'_>>) -> RegExp {
        let r = RegExp::parse_regexp_plus(chars);

        if let Some(a) = chars.peek()
            && a.is_alphanumeric()
            && *a != '+'
            && *a != '*'
        {
            let s = RegExp::parse_regexp_plus(chars);
            Concat(Box::new(r), Box::new(s))
        } else {
            r
        }
    }

    fn parse_regexp_plus(chars: &mut Peekable<Chars<'_>>) -> RegExp {
        let r = RegExp::parse_regexp_star(chars);

        if let Some('+') = chars.peek() {
            chars.next();
            let s = RegExp::parse_regexp_star(chars);
            Plus(Box::new(r), Box::new(s))
        } else {
            r
        }
    }

    fn parse_regexp_star(chars: &mut Peekable<Chars<'_>>) -> RegExp {
        let r = RegExp::parse_regexp_paren(chars);

        if let Some('*') = chars.peek() {
            chars.next();
            Star(Box::new(r))
        } else {
            r
        }
    }

    fn parse_regexp_paren(chars: &mut Peekable<Chars<'_>>) -> RegExp {
        let curr = chars.next();
        match curr {
            Some('(') => {
                if chars.peek().is_some_and(|c| *c == ')') {
                    chars.next();
                    return Lambda;
                }

                let mut r = RegExp::parse_regexp_concat(chars);
                while chars.peek().is_some_and(|c| *c != ')') {
                    r = Concat(Box::new(r), Box::new(RegExp::parse_regexp_concat(chars)));
                }
                match chars.next() {
                    Some(')') => r,
                    a => panic!("Expected ')' after expression, got {a:?}"),
                }
            }
            Some('\\' | 'λ') => Lambda,
            Some(')') | None => unreachable!(),
            Some('*' | '+') => unreachable!(),
            Some(c) => Symbol(c),
        }
    }
}

pub trait Oracle {
    /// Ask the oracle whether `input` is a part of the language.
    fn ask(&self, input: &str, sigma: &[char]) -> bool;
}

pub trait IndexOracle {
    /// Ask the oracle whether the string at index `idx` into the sequence Sigma^* over
    /// the alphabet Sigma in lexicographic order, is a part of the language.
    fn ask_i(&self, idx: usize, sigma: &[char]) -> bool;
}

/// A N-bit bigint-like unsigned integer.
/// Implemented with `Vec<u64>`
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct UIntN {
    bits: usize,
    data: Vec<u64>,
}

impl UIntN {
    /// Create a new integer with a value of 0.
    pub fn new(bits: usize) -> UIntN {
        assert!(bits >= 1, "Cannot create an integer with less than 1 bit.");

        // TODO: check for a better way to do this?
        let len = ((bits) as f32 / 64.0).ceil();
        let data = vec![0; len as usize];

        UIntN { bits, data }
    }

    /// Set bit at index `bit` (set it to 1.)
    pub fn set_bit(&mut self, bit: usize) {
        debug_assert!(
            bit < self.bits,
            "Cannot set bit {bit} for a {}-bit integer (zero-indexed; highest bit is {}, lowest 0)",
            self.bits,
            self.bits - 1
        );

        let idx = bit / 64;
        let bit = bit % 64;
        self.data[idx] |= 1 << bit;
    }

    /// Clear bit at index `bit` (set it to 0.)
    pub fn clear_bit(&mut self, bit: usize) {
        debug_assert!(
            bit < self.bits,
            "Cannot clear bit {bit} for a {}-bit integer (zero-indexed; highest bit is {}, lowest 0)",
            self.bits,
            self.bits - 1
        );

        let idx = bit / 64;
        let bit = bit % 64;
        self.data[idx] &= u64::MAX - (1 << bit);
    }

    /// Check whether the bit at index `bit` is set or not.
    pub fn is_bit_set(&self, bit: usize) -> bool {
        debug_assert!(
            bit < self.bits,
            "Cannot check if {bit} is set for a {}-bit integer (zero-indexed; highest bit is {}, lowest 0)",
            self.bits,
            self.bits - 1
        );

        let idx = bit / 64;
        let bit = bit % 64;
        (self.data[idx] & (1 << bit)) != 0
    }

    /// Get the indices of all set bits
    pub fn get_set_bits(&self) -> FxHashSet<usize> {
        FxHashSet::from_iter((0..self.bits).filter(|i| self.is_bit_set(*i)))
    }
}

impl fmt::Display for UIntN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // self.data.iter().for_each(|v| write!(f, "{v:064b}").unwrap());
        for (i, val) in self.data.iter().rev().enumerate() {
            let res = if i == 0 && !self.bits.is_multiple_of(64) {
                write!(f, "{val:0length$b}", length = self.bits % 64)
            } else {
                write!(f, "{val:064b}")
            };
            res?
        }
        Ok(())
    }
}

/// Get all possible strings of length up to `k` over Sigma^* for the alphabet Sigma.
///
/// I.e., get all strings $$\{ \lambda \} \cup \bigcup_i=1^k\Sigma^i$$
pub fn all_strs_of_len(sigma: &[char], k: usize) -> Vec<String> {
    (0..k_and_lower(k, sigma.len())).map(|i| construct_str_at(i, sigma)).collect()
}


/// Get the "level" at which the string at index `idx` is stored.
///
/// Essentially, this is the same as the length of the string at index `idx`.
fn level(str_idx: usize, len: usize) -> usize {
    if str_idx == 0 {
        return 0;
    }

    let mut i = 0;
    let mut acc = 0;

    while str_idx >= acc {
        acc += len.pow(i as u32);
        i += 1;
    }

    i - 1
}

/// Get the index into the "level" the string at index `idx` is stored.
///
/// Essentially, if the string at `idx` is the i'th string of length k, this returns i.
fn level_idx(str_idx: usize, exp: usize, len: usize) -> usize {
    if len == 2 {
        str_idx - ((usize::MAX << exp) ^ usize::MAX)
    } else {
        str_idx - k_and_lower(exp - 1, len)
    }
}

/// Get the number of possible substrings of length less than or equal to `k`
/// over some set with cardinality `len`.
pub fn k_and_lower(k: usize, len: usize) -> usize {
    (len.pow(k as u32 + 1) - 1) / (len - 1)
}

/// Consider the sequence of all strings over Sigma^* for some alphabet Sigma of length `len`
/// in lexicographic order.
/// This function returns the index into the sequence of the string resulting from concatenating
/// the string stored at index `i` in the sequence, and the string stored at index `j` in the sequence.
pub fn concat_idxes(i: usize, j: usize, len: usize) -> usize {
    if i == 0 {
        return j;
    }
    if j == 0 {
        return i;
    }

    let i_level = level(i, len);
    let j_level = level(j, len);

    let i_level_idx = level_idx(i, i_level, len);
    let j_level_idx = level_idx(j, j_level, len);

    let level_start_idx = k_and_lower(i_level + j_level - 1, len);
    let sub_level_idx = i_level_idx * len.pow(j_level as u32);
    let in_level_idx = j_level_idx;

    level_start_idx + sub_level_idx + in_level_idx
}

/// Construct the string stored at index `idx` in the set of strings
/// over Sigma^* in lexicographic order
pub fn construct_str_at(idx: usize, sigma: &[char]) -> String {
    if idx == 0 {
        return String::new();
    }

    let mut level = level(idx, sigma.len());
    let mut level_idx = level_idx(idx, level, sigma.len());

    let mut v = vec![0; level];

    while level > 0 {
        let c = sigma[level_idx % sigma.len()];
        let rev_idx = level - 1;
        v[rev_idx] = c.try_into().unwrap();

        level_idx /= sigma.len();
        level -= 1;
    }

    String::from_utf8(v).unwrap()
}

/// Construct the "string" (vec of char indices) stored at index `idx`
/// in the set of strings over Sigma^* in lexicographic order
pub fn construct_str_idxes_at(idx: usize, sigma: &[char]) -> Vec<usize> {
    if idx == 0 {
        return vec![];
    }

    let mut level = level(idx, sigma.len());
    let mut level_idx = level_idx(idx, level, sigma.len());

    let mut v = vec![usize::MAX; level];

    while level > 0 {
        let i = level_idx % sigma.len();
        let rev_idx = level - 1;
        v[rev_idx] = i;

        level_idx /= sigma.len();
        level -= 1;
    }

    v
}

/// Construct the "string" (vec of char indices) from the actual string `s`
pub fn construct_str_idxes_from(s: &str, sigma: &[char]) -> Vec<usize> {
    if s.len() == 0 {
        return vec![];
    }

    let rev_sigma = sigma
        .iter()
        .enumerate()
        .map(|(i, x)| (x,i))
        .collect::<FxHashMap<_,_>>();

    s.chars()
        .map(|c| rev_sigma[&c])
        .collect()
}


#[cfg(test)]
mod test {
    use std::hash::{DefaultHasher, Hash};

    use super::*;

    #[test]
    fn test_uintn() {
        let mut a = UIntN::new(64);
        let mut b = UIntN::new(64);
        let mut c = UIntN::new(65);

        assert_eq!(a, b);
        assert_ne!(a, c);
        a.set_bit(10);
        assert_ne!(a, b);
        b.set_bit(10);
        assert_eq!(a, b);
        c.set_bit(10);
        assert_ne!(a, c);

        let mut hasher = DefaultHasher::new();
        assert_eq!(a.hash(&mut hasher), b.hash(&mut hasher));
        a.clear_bit(10);
        assert_eq!(a, UIntN::new(64));
    }

    #[rustfmt::skip]
    #[test]
    fn test_regex() {
        // Test that the regex parsing worked (might be less ambiguous)
        assert_eq!(RegExp::parse("(ab+b)*").to_string(),    "(a(b+b))*");
        assert_eq!(RegExp::parse("((ab)+b)*").to_string(),  "((ab)+b)*");
        assert_eq!(RegExp::parse("(0+1)*00").to_string(),   "(0+1)*00");
        assert_eq!(RegExp::parse("helloworld").to_string(), "helloworld");
        assert_eq!(RegExp::parse("((a*b*))").to_string(),   "a*b*");
        assert_eq!(RegExp::parse("()*").to_string(),        "(λ)*");
        assert_eq!(RegExp::parse("((aa*(b+bb)*bb*))").to_string(), "aa*((b+b)b)*bb*");
    }

    #[rustfmt::skip]
    #[test]
    fn test_all_strs() {
        let strs = vec![
            "",
            "0", "1",
            "00", "01", "10", "11",
            "000", "001", "010", "011",
            "100", "101", "110", "111",
        ];
        assert_eq!(all_strs_of_len(&['0','1'], 3), strs);
        let strs = vec![
            "",

            "a", "b", "c",

            "aa", "ab", "ac",
            "ba", "bb", "bc",
            "ca", "cb", "cc",

            "aaa", "aab", "aac",
            "aba", "abb", "abc",
            "aca", "acb", "acc",

            "baa", "bab", "bac",
            "bba", "bbb", "bbc",
            "bca", "bcb", "bcc",

            "caa", "cab", "cac",
            "cba", "cbb", "cbc",
            "cca", "ccb", "ccc",
        ];
        assert_eq!(all_strs_of_len(&['a','b', 'c'], 3), strs);
    }
}
