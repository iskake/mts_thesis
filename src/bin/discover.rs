use std::{cmp::max, env};

use mts::{MTSOpt, util::{IndexOracle, construct_str_at, k_and_lower, table::FnOracle}};

fn contains_x_exactly_n_from_end(x: usize, n: usize, num: usize, base: usize) -> bool {
    ((num / base.pow(n as u32)) % base) == x
}

fn contains_eactly_n_1s_followedby_lt_m_0s(w: usize, n: usize, m: usize, len: usize, base: usize) -> bool {
    let mut num = w;

    let mut zeroes = 0;
    let mut consecutive_ones = 0;

    for _ in 0..len {
        if num % base == 0 {
            zeroes += 1;
            consecutive_ones = 0;
        }
        if zeroes >= m {
            return false;
        } else if num % base == 1 {
            consecutive_ones += 1;
        }
        num /= base;
        if consecutive_ones == n && num % base == 0 {
            return true
        }
    }
    false
}

fn error_exit(msg: &str) -> ! {
    eprintln!("{}\n", msg);
    eprintln!("usage: discover l_n|l_nm <n> <m|sigma_size> ...");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        error_exit("Invalid arguments! expected language (either `l_n` or `l_nm`) and parameters (one or two integers)");
    }

    let n;
    let m;
    let sigma_size;

    if args.len() != 4 {
        error_exit("Expected two integer and parameters after language.");
    }

    let f: Box<dyn Fn(usize, usize) -> bool> = match args[1].to_lowercase().as_str() {
        "l_n" | "ln" | "l(n)" => {
            println!("Language: L(n) with parameters n and sigma_size");

            (n,sigma_size) = match (args[2].parse(), args[3].parse()) {
                (Ok(x), Ok(y)) if y <= 16 && y >= 2 => (x,y),
                _ => error_exit("Expected integer and parameters n and sigma_size (2 <= size <= 16) after language."),
            };
            m = 0;

            Box::new(|j: usize, _: usize| contains_x_exactly_n_from_end(1, n, j, sigma_size))
        },
        "l_nm" | "l_n,m" | "l_{n,m}"| "ln,m" | "lnm" | "l(n,m)" | "l(nm)" => {
            println!("Language: L(n,m) with parameters n and m");

            (n,m) = match (args[2].parse(), args[3].parse()) {
                (Ok(x), Ok(y)) if y >= 1 && x >= 1 => (x,y),
                _ => error_exit("Expected integer and parameters n and m (> 0) after language."),
            };
            sigma_size = 2;

            Box::new(|j: usize, j_len: usize| contains_eactly_n_1s_followedby_lt_m_0s(j, n, m, j_len, sigma_size))
        },
        _ => {
            error_exit("Expected language, either l_n or l_nm.");
        }
    };

    let sigma = (0usize..16)
        .map(|x| char::from_digit(x as u32, 16).unwrap())
        .take(sigma_size)
        .collect::<Vec<_>>();

    let table = FnOracle(f);

    for i in 2..=23 {
        let k = i;

        println!("Running with k = {k} (note: with k of {k}, {} strings are required...?)", k_and_lower(k+3, sigma.len()));

        let mts_dfa = mts::mts_otf(&sigma, k, &table, MTSOpt::Index);

        match mts_dfa {
            Ok(mts_dfa) => {
                println!("Constructed dfa:");
                mts_dfa.print_dfa();

                let size = k_and_lower(max(n+m+1, k+k-1), sigma.len());

                let mut success = true;
                for i in 0..size {
                    if mts_dfa.ask_i(i, &sigma) != table.ask_i(i, &sigma) {
                        println!("dfa failed for {} (dfa: {}, oracle: {}): ({mts_dfa:?})", construct_str_at(i, &sigma), mts_dfa.ask_i(i, &sigma), table.ask_i(i, &sigma));
                        success = false;
                        break;
                    }
                }
                if success {
                    println!("\n!!!!\n!!!!\n!!!!Successfully created dfa with same language (supposedly)\n!!!!\n!!!!\n");
                    break;
                }
            },
            Err(e) => println!("Failed to construct dfa: {e:?}"),
        }
    }
}