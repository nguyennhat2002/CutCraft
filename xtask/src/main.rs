mod build;
mod package;
mod protect;
mod test;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- protect [--root <dir>] [--rules <file>] [--all-files] [--verbose] [--expect-origin <url>]";

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(command) = arguments.next() else {
        eprintln!("xtask is scaffold only; build, test, and package tasks are not implemented yet");
        eprintln!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    if command != "protect" {
        eprintln!("xtask is scaffold only; build, test, and package tasks are not implemented yet");
        eprintln!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let mut root: Option<PathBuf> = None;
    let mut rule_files: Vec<PathBuf> = Vec::new();
    let mut verbose = false;
    let mut all_files = false;
    let mut expect_origin: Option<String> = None;
    let arguments: Vec<String> = arguments.collect();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--root" => {
                index += 1;
                root = Some(PathBuf::from(
                    arguments.get(index).cloned().unwrap_or_default(),
                ));
            }
            "--rules" => {
                index += 1;
                rule_files.push(PathBuf::from(
                    arguments.get(index).cloned().unwrap_or_default(),
                ));
            }
            "--expect-origin" => {
                index += 1;
                expect_origin = Some(arguments.get(index).cloned().unwrap_or_default());
            }
            "--all-files" => all_files = true,
            "--verbose" => verbose = true,
            other => {
                eprintln!("xtask protect: unknown argument `{other}`\n{USAGE}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }

    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let mut rules = Vec::new();
    if rule_files.is_empty() {
        rule_files.push(protect::default_rules_path());
    }
    for file in &rule_files {
        match protect::RuleSet::load(file) {
            Ok(set) => rules.extend(set.rules),
            Err(error) => {
                eprintln!("xtask protect: {error}");
                return ExitCode::from(2);
            }
        }
    }
    let rule_set = protect::RuleSet { rules };

    // Tracked files are the publication surface; `--all-files` also walks a scratch
    // directory, which is how the negative tests and local audits run.
    let paths = if all_files {
        let mut paths = Vec::new();
        protect::walk(&root, &root, &mut paths);
        paths
    } else {
        match protect::tracked_paths(&root) {
            Ok(paths) => paths,
            Err(error) => {
                eprintln!("xtask protect: {error}");
                return ExitCode::from(2);
            }
        }
    };

    let report = {
        let mut skips = std::collections::HashSet::new();
        for file in &rule_files {
            if let Ok(resolved) = file.canonicalize() {
                skips.insert(resolved);
            }
        }
        protect::scan(&paths, &rule_set, Some(&root), &skips)
    };
    protect::print_findings(&report, verbose, &mut std::io::stdout());

    if let Some(expected) = &expect_origin {
        match protect::origin_url(&root) {
            Some(url) if protect::normalize_origin(&url) == protect::normalize_origin(expected) => {
                println!("protect: origin verified");
            }
            other => {
                eprintln!(
                    "xtask protect: origin {} (expected {expected})",
                    other.map_or("is absent".to_owned(), |url| format!("is `{url}`"))
                );
                return ExitCode::from(1);
            }
        }
    }

    if report.findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
