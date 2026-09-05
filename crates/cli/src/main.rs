//! Quick manual query tool for exercising `hl7pet-core`'s scanner, parser,
//! query executor, and hierarchy navigator (specs 005-008) from the command
//! line. Not a formal roadmap feature — a dev tool, not tracked as its own
//! spec.
//!
//! Usage: hl7pet <message-file> <path-expr> [--first] [--profile <file>] [--located]
//!
//! Exit codes: 0 = match(es) found, 1 = no match, 2 = usage/read/scan/parse/
//! profile/query error (grep-like convention).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let program = "hl7pet";

    let mut positional: Vec<&String> = Vec::new();
    let mut first_only = false;
    let mut located = false;
    let mut profile_path: Option<&String> = None;
    let mut arg_iter = args[1..].iter();
    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--first" => first_only = true,
            "--located" => located = true,
            "--profile" => {
                profile_path = arg_iter.next();
                if profile_path.is_none() {
                    eprintln!("error: --profile requires a value");
                    return ExitCode::from(2);
                }
            }
            _ => positional.push(arg),
        }
    }

    let [file_path, path_expr] = positional[..] else {
        eprintln!("usage: {program} <message-file> <path-expr> [--first] [--profile <file>] [--located]");
        eprintln!();
        eprintln!("  <message-file>  path to a raw HL7 v2 message file");
        eprintln!("  <path-expr>     a PATH expression, e.g. \"PID-5.1\", \"OBX[2]-5\", or");
        eprintln!("                  \"OBR[1] -> OBX-5\" (hierarchy, requires --profile)");
        eprintln!("  --first         print only the first matched value (getFirstValue-style)");
        eprintln!("  --profile <file>  a segmentDefinition JSON profile, for hierarchy PATHs");
        eprintln!("  --located       print each value with its 1-based source line (non-hierarchy PATHs only, spec 1000)");
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

    if located {
        if compiled.child.is_some() {
            eprintln!("error: --located does not support hierarchy PATHs (\"->\"), spec 1000 FR-009 scope");
            return ExitCode::from(2);
        }
        let values = if first_only {
            match hl7pet_core::first_located(&scan_result, &compiled) {
                Ok(v) => v.into_iter().collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("query error: {e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            match hl7pet_core::execute_located(&scan_result, &compiled) {
                Ok(v) => v.into_iter().flatten().collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("query error: {e}");
                    return ExitCode::from(2);
                }
            }
        };

        return if values.is_empty() {
            println!("(no match)");
            ExitCode::from(1)
        } else {
            for lv in &values {
                println!("line {}: {}", lv.line, lv.value);
            }
            ExitCode::SUCCESS
        };
    }

    let profile = match profile_path {
        Some(path) => {
            let profile_json = match std::fs::read_to_string(path) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("error reading profile {path}: {e}");
                    return ExitCode::from(2);
                }
            };
            match hl7pet_core::HierarchyProfile::from_json(&profile_json) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("profile error: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        None => None,
    };

    let values = match hl7pet_core::execute_hierarchy(&scan_result, &compiled, profile.as_ref()) {
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
