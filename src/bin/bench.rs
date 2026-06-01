use std::env;
use std::{io::Write, time::Instant};
use mts::util::fa::DFA;

use mts::{
    MTSOpt,
    util::{
        IndexOracle, Oracle, all_strs_of_len, k_and_lower, table::{ResponseTable, StrResponseWrapper}
    },
};

enum MTSType {
    Naive,
    OnTheFly,
}
use MTSType::*;

fn error_exit(msg: &str) -> ! {
    eprintln!("{}\n", msg);
    eprintln!("usage: bench <lang> <sigma_size> <out_dir>");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        error_exit("Invalid arguments!");
    }

    enum Lang {
        L1, // (0+1+...+n)*0^{k-2}
        L2, // ((0+1+...+n)*0)*1^{k-2}
        L3, // 1^{k-2}
    }

    let lang = match args[1].to_lowercase().as_str() {
        "l_1" => Lang::L1,
        "l_2" => Lang::L2,
        "l_3" => Lang::L3,
        x => error_exit(&format!("Invalid language selection: expected `l_1`, `l_2`, or `l_3`, got {x}")),
    };

    let sigma_size: usize = match args[2].parse() {
        Ok(x@2..=9) => x,
        Ok(x@0..=1) => {
            error_exit(&format!("Invalid alpahbet size: specified size is too small! expected natural number in {{2, 3, ..., 9}}, got {x}\n"));
        },
        Ok(x@10..) => {
            error_exit(&format!("Invalid alpahbet size: specified size is too large! expected natural number in {{2, 3, ..., 9}}, got {x}\n"));
        },
        Err(_) => {
            error_exit(&format!("Invalid alphabet size! expected natural number, got `{}`.\n", args[1]));
        },
    };

    let out_dir = &args[3];

    let mut benches: Vec<(_, Vec<char>, _, _)> = vec![];

    let base_sigma = (0usize..10)
        .map(|x| char::from_digit(x as u32, 10).unwrap())
        .take(sigma_size)
        .collect::<Vec<_>>();

    let mut base_name = match base_sigma.len() { 
        2 => String::from("(0 \\cup 1)^*"),
        3 => String::from("(0 \\cup 1 \\cup 2)^*"),
        4 => String::from("(0 \\cup 1 \\cup 2 \\cup 3)^*"),
        _ => format!("(0 \\cup 1 \\cup 2 \\cup \\dots \\cup {})^*", base_sigma.len() - 1)
    };

    let mut base_regex = String::from("(");
    for i in 0..base_sigma.len() {
        if i % 2 == 0 {
            base_regex.push_str(&format!("({i}"));
        } else {
            base_regex.push_str(&format!("+{i}"));

            if i != base_sigma.len() - 1 {
                base_regex = format!("({base_regex}))+")
            }
        }
        if i == base_sigma.len() - 1 {
            base_regex.push_str(")");
        }
    }
    base_regex.push_str(")*");

    match lang {
        Lang::L3 => {
            base_name = format!("$1^X$");
            base_regex = format!("1");
        }
        Lang::L2 => {
            base_name = format!("$({base_name}0)^*");
            base_regex = format!("({base_regex}0)*");
        }
        Lang::L1 => {
            base_name = format!("${base_name}");
        }
    }

    println!("base regex: {base_regex}, sigma: {base_sigma:?}");

    let name = match lang {
        Lang::L1 => format!("{base_name}0"),
        Lang::L2 => format!("{base_name}1"),
        Lang::L3 => format!("{}", base_name.replace("X", "{1}")),
    }; 
    let sigma = base_sigma.clone();
    let k = match lang {
        Lang::L3 => 3,
        _ => 2,
    };
    let regex = match lang {
        Lang::L3 => String::from("1"),
        Lang::L1 => format!("{base_regex}0"),
        _ => base_regex.clone(),
    };

    // From anecdotal evidence
    let iters = match &base_sigma.len() {
        0..=2 => 13,
        3 => 8,
        4 => 7,
        5 => 6,
        6 => 5,
        7 => 5,
        8 => 4,
        9 => 4,
        _ => 4,
    };

    println!("regex: {regex}");
    benches.push((name, sigma, k, regex));

    let push_sym = match lang {
        Lang::L2 | Lang::L3 => '1',
        _ => '0',
    };

    let start_iter = match lang {
        Lang::L3 => 3,
        _ => 2,
    };

    for i in start_iter..=iters {
        let mut suffix = String::with_capacity(i);

        let start_i = match lang {
            Lang::L3 => 3,
            Lang::L2 => 2,
            _ => 1,
        };

        for _ in start_i..i {
            suffix.push(push_sym);
        }

        let sigma = base_sigma.clone();
        let k = i;
        let name = match lang {
            Lang::L3 => format!("{}", base_name.replace("X", &format!("{{{}}}", (k - 2)))),
            _ => format!("{base_name}{suffix}$"),
        };
        let regex = format!("{base_regex}{suffix}");
        benches.push((name, sigma, k, regex));
    }

    let mut naive_results_idx = Vec::with_capacity(benches.len());
    let mut naive_results_str = Vec::with_capacity(benches.len());
    let mut naive_results_str_idx = Vec::with_capacity(benches.len());
    let mut naive_results_none = Vec::with_capacity(benches.len());

    let mut otf_results_idx = Vec::with_capacity(benches.len());
    let mut otf_results_str = Vec::with_capacity(benches.len());
    let mut otf_results_str_idx = Vec::with_capacity(benches.len());
    let mut otf_results_none = Vec::with_capacity(benches.len());

    println!("Starting tests!");
    println!("(Note: the first test is done as a 'warm-up' (so things like caching doesn't mess with the later tests) and is not actually counted)");

    let mut to_write = String::new();

    macro_rules! run_str {
        ($wrapper:expr, $sigma:expr) => {
            //
            // STRING
            //
            println!("  String");

            oracle_runner!(
                &$wrapper,
                MTSOpt::String,
                ask,
                &all_strs_of_len($sigma, k),
                naive_results_str,
                otf_results_str
            );
        };
    }

    macro_rules! run_str_constr {
        ($wrapper:expr, $sigma:expr) => {
            //
            // STRING+INDEX (construction from index)
            //
            println!("  String construction");

            oracle_runner!(
                &$wrapper,
                MTSOpt::StringIndex,
                ask,
                &all_strs_of_len($sigma, k),
                naive_results_str_idx,
                otf_results_str_idx
            );
        };
    }

    macro_rules! run_unopt {
        ($wrapper:expr, $sigma:expr) => {
            //
            // UNOPT MTS 
            //
            println!("  Pre-speedup");

            oracle_runner!(
                &$wrapper,
                MTSOpt::None,
                ask,
                &all_strs_of_len($sigma, k),
                naive_results_none,
                otf_results_none
            );
        };
    }

    macro_rules! run_entire {
        ($run_str_0:ident, $run_str_1:ident, $run_str_2:ident) => {
            for (j, (name, sigma, k, regex)) in benches.iter().enumerate().map(|(i, x)| (i, x.clone())) {
                let sigma: &[char] = sigma.as_ref();

                println!("{name} (k = {k})");
                to_write.push_str(&format!("{name} & ($k={k}$) "));

                let thing_k = if k <= 2 { 2 * 2 } else { k + (k - 1) };

                let mut to_write_naive = String::new();
                let mut to_write_otf = String::new();

                macro_rules! oracle_runner {
                    (
                        $oracle:expr,
                        $mts_opt:expr,
                        $ask_fn:ident,
                        $looper:expr,
                        $naive_results_table:expr,
                        $otf_results_table:expr
                    ) => {
                        for mts_type in [Naive, OnTheFly] {
                            let mts_fn = if let Naive = mts_type {
                                if let MTSOpt::None = $mts_opt {
                                    mts::mts_naive_unopt
                                } else {
                                    mts::mts_naive
                                }
                            } else {
                                if let MTSOpt::None = $mts_opt {
                                    mts::mts_otf_unopt
                                } else {
                                    mts::mts_otf
                                }
                            };

                            let before = Instant::now();
                            let mts_dfa = mts_fn(sigma, k, $oracle, $mts_opt);
                            let after = before.elapsed();

                            let mts_dfa = mts_dfa.unwrap();

                            for i in $looper {
                                assert_eq!(
                                    mts_dfa.$ask_fn(i, sigma),
                                    $oracle.$ask_fn(i, sigma),
                                    "dfa failed for {i}: ({mts_dfa:?})"
                                );
                            }

                            assert!(
                                mts_dfa.n_states <= k,
                                "expected {k} (or less) states, constructed dfa (table) has {} ({mts_dfa:#?})",
                                mts_dfa.n_states
                            );

                            // Use thousands separators for the numbers
                            // (For some reason you can't do this easily in rust...)

                            // Source - https://stackoverflow.com/a/67834588
                            // Posted by Kaplan, modified by community. See post 'Timeline' for change history
                            // Retrieved 2026-04-22, License - CC BY-SA 4.0
                            let s = after.as_nanos()
                                .to_string()
                                .as_bytes()
                                .rchunks(3)
                                .rev()
                                .map(std::str::from_utf8)
                                .collect::<Result<Vec<&str>, _ >>()
                                .unwrap()
                                .join("~");

                            match mts_type {
                                Naive    => {
                                    println!("    {:?} (naive)", after);
                                    $naive_results_table.push((k, after.as_nanos()));
                                    to_write_naive.push_str(&format!("&{} ", s));
                                }
                                OnTheFly => {
                                    println!("    {:?} (on-the-fly)", after);
                                    $otf_results_table.push((k, after.as_nanos()));
                                    to_write_otf.push_str(&format!("&{} ", s));
                                }
                            }
                        }
                    };
                }

                //
                // PRE-SPEEDUP, STRING, STRING INDEX
                //
                let before = Instant::now();
                let wrapper = StrResponseWrapper::from_oracle(
                    DFA::from((regex.as_ref(), sigma.as_ref())),
                    sigma,
                    thing_k,
                );
                let after = before.elapsed();
                println!("  ({:?} to construct oracle)", after);

                $run_str_0!(wrapper, sigma);
                $run_str_1!(wrapper, sigma);
                $run_str_2!(wrapper, sigma);

                drop(wrapper);

                //
                // INDEX
                //
                println!("  Index");

                let before = Instant::now();
                let table = ResponseTable::from_idx_oracle(
                    DFA::from((regex.as_ref(), sigma.as_ref())),
                    sigma,
                    thing_k,
                );
                let after = before.elapsed();
                println!("  ({:?} to construct oracle)", after);

                oracle_runner!(
                    &table,
                    MTSOpt::Index,
                    ask_i,
                    0..k_and_lower(k + 2, sigma.len()),
                    naive_results_idx,
                    otf_results_idx
                );
                drop(table);

                to_write.push_str(&to_write_naive);
                to_write.push_str(&to_write_otf);
                to_write.push_str("\\\\\n");

                // Ignore first line to account for os / processor things
                if j == 0 {
                    to_write = String::new();
                }
            }
        };
    }

    run_entire!(run_unopt, run_str, run_str_constr);

    let table_path = match lang {
        Lang::L3 => &format!("{out_dir}table_{base_regex}sigma_siz={}.tex", sigma_size),
        _ => &format!("{out_dir}table_{base_regex}0^k.tex"),
    };

    let res = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .append(false)
        .truncate(true)
        .open(table_path);

    match res {
        Ok(mut file) => file.write_all(to_write.as_bytes()).unwrap(),
        Err(_) => std::fs::write(table_path, to_write.as_bytes()).unwrap(),
    }

    println!("All good!");
}
