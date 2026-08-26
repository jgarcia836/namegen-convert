mod json;
mod model;
mod ngj;
mod ngt;
mod sample;

use model::{finalize, Issue, Severity};
use std::env;
use std::fs;
use std::process;

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Ngt,
    Ngj,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = if args.first().map(String::as_str) == Some("sample") {
        run_sample(args[1..].to_vec())
    } else {
        run(args)
    };
    if let Err(message) = result {
        eprintln!("error: {}", message);
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut lenient = false;
    let mut from: Option<Format> = None;
    let mut to: Option<Format> = None;
    let mut positional = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lenient" => lenient = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--from" => {
                i += 1;
                let value = args.get(i).ok_or("--from requires a value (ngt or ngj)")?;
                from = Some(parse_format(value)?);
            }
            "--to" => {
                i += 1;
                let value = args.get(i).ok_or("--to requires a value (ngt or ngj)")?;
                to = Some(parse_format(value)?);
            }
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    if positional.len() != 2 {
        print_help();
        return Err("expected exactly one input path and one output path".to_string());
    }

    let input_path = &positional[0];
    let output_path = &positional[1];

    let from = match from {
        Some(f) => f,
        None => format_from_extension(input_path)
            .ok_or_else(|| format!("cannot infer input format from '{}'; pass --from", input_path))?,
    };
    let to = match to {
        Some(f) => f,
        None => format_from_extension(output_path)
            .ok_or_else(|| format!("cannot infer output format from '{}'; pass --to", output_path))?,
    };

    let source = fs::read_to_string(input_path).map_err(|e| format!("failed to read '{}': {}", input_path, e))?;

    let (mut doc, mut issues) = match from {
        Format::Ngt => ngt::parse(&source),
        Format::Ngj => ngj::parse(&source),
    }
    .map_err(|fatal| fatal.message)?;

    finalize(&mut doc, &mut issues);
    report_issues(&issues, lenient)?;

    let mut output = match to {
        Format::Ngt => ngt::write(&doc),
        Format::Ngj => ngj::write(&doc),
    };
    if !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(output_path, output).map_err(|e| format!("failed to write '{}': {}", output_path, e))?;

    Ok(())
}

fn run_sample(args: Vec<String>) -> Result<(), String> {
    let mut lenient = false;
    let mut from: Option<Format> = None;
    let mut count: u32 = 1;
    let mut positional = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lenient" => lenient = true,
            "--from" => {
                i += 1;
                let value = args.get(i).ok_or("--from requires a value (ngt or ngj)")?;
                from = Some(parse_format(value)?);
            }
            "--count" => {
                i += 1;
                let value = args.get(i).ok_or("--count requires a number")?;
                count = value.parse::<u32>().map_err(|_| format!("'{}' is not a valid count", value))?;
                if count == 0 {
                    return Err("--count must be at least 1".to_string());
                }
            }
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    if positional.len() != 1 {
        return Err("expected exactly one input path".to_string());
    }
    let input_path = &positional[0];

    let from = match from {
        Some(f) => f,
        None => format_from_extension(input_path)
            .ok_or_else(|| format!("cannot infer input format from '{}'; pass --from", input_path))?,
    };

    let source = fs::read_to_string(input_path).map_err(|e| format!("failed to read '{}': {}", input_path, e))?;
    let (mut doc, mut issues) = match from {
        Format::Ngt => ngt::parse(&source),
        Format::Ngj => ngj::parse(&source),
    }
    .map_err(|fatal| fatal.message)?;

    finalize(&mut doc, &mut issues);
    report_issues(&issues, lenient)?;

    let mut rng = sample::Rng::from_entropy();
    for _ in 0..count {
        let name = sample::sample(&doc, &mut rng)?;
        println!("{}", name);
    }

    Ok(())
}

fn report_issues(issues: &[Issue], lenient: bool) -> Result<(), String> {
    if issues.is_empty() {
        return Ok(());
    }

    let has_fatal = issues.iter().any(|i| matches!(i.severity, Severity::Fatal));
    for issue in issues {
        let label = match issue.severity {
            Severity::Fatal => "error",
            Severity::Semantic if lenient => "warning",
            Severity::Semantic => "error",
        };
        eprintln!("{}: {}", label, issue.message);
    }

    if has_fatal {
        Err(format!("{} issue(s) found and cannot be bypassed with --lenient", issues.len()))
    } else if lenient {
        Ok(())
    } else {
        Err(format!(
            "{} issue(s) found; rerun with --lenient to convert anyway",
            issues.len()
        ))
    }
}

fn parse_format(value: &str) -> Result<Format, String> {
    match value {
        "ngt" => Ok(Format::Ngt),
        "ngj" => Ok(Format::Ngj),
        other => Err(format!("unknown format '{}'; expected 'ngt' or 'ngj'", other)),
    }
}

fn format_from_extension(path: &str) -> Option<Format> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "ngt" | "txt" => Some(Format::Ngt),
        "ngj" | "json" => Some(Format::Ngj),
        _ => None,
    }
}

fn print_help() {
    println!("namegen-convert - convert between .ngt and .ngj name generator files");
    println!();
    println!("usage:");
    println!("  namegen-convert [--lenient] [--from ngt|ngj] [--to ngt|ngj] <input> <output>");
    println!("  namegen-convert sample [--lenient] [--from ngt|ngj] [--count N] <input>");
    println!();
    println!("by default the conversion is strict: an undefined start category, a");
    println!("placeholder referencing an unknown category, or a duplicate category");
    println!("definition all stop the conversion. pass --lenient to downgrade those");
    println!("to warnings and convert anyway.");
    println!();
    println!("'sample' parses a grammar and prints N generated names (default 1) by");
    println!("expanding its start category, without writing a converted file.");
}
