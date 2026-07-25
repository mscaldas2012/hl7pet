//! Quick manual query tool for exercising `hl7pet-core`'s scanner, parser,
//! and query executor (specs 005-007) from the command line. Not a formal
//! roadmap feature — a dev tool, not tracked as its own spec.
//!
//! Usage: hl7pet <message-file> <path-expr> [--first]
//!
//! Exit codes: 0 = match(es) found, 1 = no match, 2 = usage/read/scan/parse/
//! query error (grep-like convention).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let program = "hl7pet";

    let positional: Vec<&String> = args[1..].iter().filter(|a| a.as_str() != "--first").collect();
    let first_only = args[1..].iter().any(|a| a == "--first");

    let [file_path, path_expr] = positional[..] else {
        eprintln!("usage: {program} <message-file> <path-expr> [--first]");
        eprintln!();
        eprintln!("  <message-file>  path to a raw HL7 v2 message file");
        eprintln!("  <path-expr>     a PATH expression, e.g. \"PID-5.1\" or \"OBX[2]-5\"");
        eprintln!("  --first         print only the first matched value (getFirstValue-style)");
        return ExitCode::from(2);
    };

    let message = match std::fs::read_to_string(file_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error reading {file_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let scan_result = match hl7pet_core::scan(&message) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("scan error: {e}");
            return ExitCode::from(2);
        }
    };

    let compiled = match hl7pet_core::parse(path_expr) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("parse error: {e}");
            return ExitCode::from(2);
        }
    };

    if compiled.child.is_some() {
        eprintln!(
            "warning: hierarchy PATH (\"->\") is not evaluated yet (spec 008) — \
             only the parent segment/field is queried"
        );
    }

    let values = match hl7pet_core::execute(&scan_result, &compiled) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("query error: {e}");
            return ExitCode::from(2);
        }
    };

    if first_only {
        match values.first().and_then(|reps| reps.first()) {
            Some(v) => {
                println!("{v}");
                ExitCode::SUCCESS
            }
            None => {
                println!("(no match)");
                ExitCode::from(1)
            }
        }
    } else if values.is_empty() {
        println!("(no match)");
        ExitCode::from(1)
    } else {
        for (i, reps) in values.iter().enumerate() {
            println!("[{}] {}", i + 1, reps.join(" ~ "));
        }
        ExitCode::SUCCESS
    }
}
