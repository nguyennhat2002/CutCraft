//! Detect-only research-contamination guardrail for the native CutCraft repository.
//!
//! The guard reads the tracked file list and file contents, compares them against
//! rule files, and reports violations. It never deletes, moves, rewrites or
//! restages anything: the only side effect is an exit code.
//!
//! Prohibited path prefixes are stored as SHA-256 digests so that the rule set
//! itself does not disclose which research paths once existed, and violation
//! output is digest-only unless `--verbose` is passed locally.
//!
//! # What the default scan covers
//!
//! - Tracked paths (`git ls-files`), matched against every normalised ancestor so
//!   one stored prefix digest also catches anything added inside that directory.
//! - Tracked file bytes, matched case-insensitively against generic content rules.
//! - `--expect-origin` additionally refuses to pass in a checkout whose `origin`
//!   is not the official repository.
//!
//! # What it deliberately does not cover
//!
//! - Ordinary untracked files, unless `--all-files` is passed.
//! - Anything under `target/` or `.git/`: [`walk`] skips both, so ignored build
//!   scratch is never inspected, in CI or during a local hygiene sweep.
//! - The inside of compressed payloads such as `.docx` or `.zip`; only the stored
//!   bytes are searched, never their members.
//! - Git history, other refs, or any other directory in the filesystem.
//! - Identifiers that no public rule describes. The public rules are a known-class
//!   filter plus a locally held private file; a clean result is not proof that no
//!   private or sensitive string exists, and it does not replace source review or
//!   keeping research in a separate workspace.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest as _, Sha256};

/// Normalised path digest used for both rule storage and violation reporting.
pub fn path_digest(path: &str) -> String {
    let normalised = path.replace('\\', "/").to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A prohibited path prefix or exact path, held as a digest.
    PathDigest,
    /// A prohibited literal substring in a tracked file's bytes.
    Content,
}

#[derive(Clone)]
pub struct Rule {
    pub id: String,
    pub kind: Kind,
    /// For [`Kind::PathDigest`]: lowercase hex digest. For [`Kind::Content`]: bytes.
    pub needle: Vec<u8>,
}

pub struct RuleSet {
    pub rules: Vec<Rule>,
}

impl RuleSet {
    /// Rules are line based: `# comment`, blank lines, `path <sha256>` and
    /// `content <literal>`. Anything else is a parse error rather than a silently
    /// ignored line, so a mistyped rule cannot weaken the guard unnoticed. There is
    /// no `allow` exemption: the rules are written as path digests and markers that
    /// no legitimate native file carries, which is what keeps live Cargo package
    /// names and the RULE 000 prohibition prose out of the finding list.
    pub fn load(path: &Path) -> Result<RuleSet, String> {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut rules = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim_end_matches('\r');
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (keyword, value) = trimmed.split_once(char::is_whitespace).ok_or_else(|| {
                format!(
                    "{}:{}: expected `path <digest>` or `content <text>`",
                    path.display(),
                    number + 1
                )
            })?;
            let id = format!(
                "{}:{}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                number + 1
            );
            match keyword {
                "path" => rules.push(Rule {
                    id,
                    kind: Kind::PathDigest,
                    needle: value.trim().as_bytes().to_vec(),
                }),
                "content" => rules.push(Rule {
                    id,
                    kind: Kind::Content,
                    needle: value.trim().as_bytes().to_vec(),
                }),
                other => {
                    return Err(format!(
                        "{}:{}: unknown rule kind `{other}`",
                        path.display(),
                        number + 1
                    ))
                }
            }
        }
        Ok(RuleSet { rules })
    }
}

#[derive(Default)]
pub struct Finding {
    pub rule: String,
    pub path: String,
    pub path_digest: String,
    pub rule_digest: String,
}

pub struct Report {
    pub scanned: usize,
    pub findings: Vec<Finding>,
}

/// Every normalised ancestor of a tracked path, so one stored prefix digest
/// matches the directory and everything inside it.
fn ancestors(path: &str) -> Vec<String> {
    let normalised = path.replace('\\', "/").to_ascii_lowercase();
    let mut out = vec![normalised.clone()];
    let mut head = normalised.as_str();
    while let Some(index) = head.rfind('/') {
        head = &head[..index];
        out.push(head.to_owned());
    }
    out
}

/// `skip` holds already-canonicalised files that must not be scanned: the rule
/// files contain the very literals the guard looks for, so scanning them would
/// make the detector report itself.
pub fn scan(
    paths: &[String],
    rules: &RuleSet,
    root: Option<&Path>,
    skip: &HashSet<PathBuf>,
) -> Report {
    let mut findings = Vec::new();
    for path in paths {
        let digests: HashSet<String> = ancestors(path).iter().map(|a| path_digest(a)).collect();
        for rule in &rules.rules {
            if rule.kind == Kind::PathDigest {
                let needle = String::from_utf8_lossy(&rule.needle).to_ascii_lowercase();
                if digests.contains(needle.as_str()) {
                    findings.push(Finding {
                        rule: rule.id.clone(),
                        path: path.clone(),
                        path_digest: path_digest(path),
                        rule_digest: needle,
                    });
                }
                continue;
            }
            let absolute = match root {
                Some(root) => root.join(path),
                None => PathBuf::from(path),
            };
            let resolved = absolute.canonicalize().unwrap_or_else(|_| absolute.clone());
            if skip.contains(&resolved) {
                continue;
            }
            let Ok(bytes) = fs::read(&absolute) else {
                continue;
            };
            // Case-insensitive so a rule cannot be dodged by capitalisation.
            let haystack = bytes.to_ascii_lowercase();
            let needle = rule.needle.to_ascii_lowercase();
            if !needle.is_empty()
                && haystack
                    .windows(needle.len())
                    .any(|window| window == needle.as_slice())
            {
                findings.push(Finding {
                    rule: rule.id.clone(),
                    path: path.clone(),
                    path_digest: path_digest(path),
                    rule_digest: path_digest(&String::from_utf8_lossy(&rule.needle)),
                });
            }
        }
    }
    Report {
        scanned: paths.len(),
        findings,
    }
}

pub fn tracked_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files", "-z"])
        .output()
        .map_err(|error| format!("git ls-files failed: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_owned())
        .collect())
}

pub fn walk(root: &Path, directory: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .map(|name| name == ".git" || name == "target")
                .unwrap_or(false)
            {
                continue;
            }
            walk(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.display().to_string().replace('\\', "/"));
        }
    }
}

/// Prints digest-only findings, or real paths under `--verbose` for local use.
pub fn print_findings(report: &Report, verbose: bool, stream: &mut dyn Write) {
    for finding in &report.findings {
        if verbose {
            let _ = writeln!(stream, "PROHIBITED {} at {}", finding.rule, finding.path);
        } else {
            let _ = writeln!(
                stream,
                "PROHIBITED rule={} path-sha256={}@{}",
                finding.rule,
                &finding.path_digest[..12.min(finding.path_digest.len())],
                &finding.rule_digest[..12.min(finding.rule_digest.len())]
            );
        }
    }
    let _ = writeln!(
        stream,
        "protect: scanned {} file(s), {} violation(s)",
        report.scanned,
        report.findings.len()
    );
}

pub fn default_rules_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("rules/protected-paths.sha256")
}

/// The configured `origin` URL, used to refuse work in a checkout that is not the
/// official native repository.
pub fn origin_url(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &root.display().to_string(),
            "remote",
            "get-url",
            "origin",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Reduces an HTTPS origin to the repository identity it names, so the `.git`
/// suffix that a manual clone carries cannot disagree with the suffix-free URL
/// `actions/checkout@v4` writes into CI. Only the trailing `/` and `.git` are
/// dropped: a value that omits the owner, such as `CutCraft`, stays unequal to
/// the full URL and still fails the check closed.
pub fn normalize_origin(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    trimmed.strip_suffix(".git").unwrap_or(trimmed).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cutcraft-protect-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_digest_of_a_prohibited_prefix_matches_that_directory_and_its_children() {
        let dir = temp_dir("prefix");
        fs::write(dir.join("notes.md"), b"benign").expect("write");
        fs::create_dir_all(dir.join("inner")).expect("mkdir");
        fs::write(dir.join("inner/notes.md"), b"benign").expect("write");
        let banned = "inner";
        let rules = RuleSet {
            rules: vec![Rule {
                id: "test/path".to_owned(),
                kind: Kind::PathDigest,
                needle: path_digest(banned).into_bytes(),
            }],
        };
        let report = scan(
            &["notes.md".to_owned(), "inner/notes.md".to_owned()],
            &rules,
            Some(&dir),
            &HashSet::new(),
        );
        assert_eq!(
            report.findings.len(),
            1,
            "only the child of the banned prefix matches"
        );
        assert_eq!(report.findings[0].path, "inner/notes.md");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn content_rules_match_bytes_and_never_mutate_the_scanned_file() {
        let dir = temp_dir("content");
        let file = dir.join("report.md");
        let payload = b"benign preamble SYNTHETIC_TOKEN_9995 trailing text";
        fs::write(&file, payload).expect("write");
        let rules = RuleSet {
            rules: vec![Rule {
                id: "test/content".to_owned(),
                kind: Kind::Content,
                needle: b"SYNTHETIC_TOKEN_9995".to_vec(),
            }],
        };
        let report = scan(
            &["report.md".to_owned()],
            &rules,
            Some(&dir),
            &HashSet::new(),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            fs::read(&file).expect("re-read"),
            payload,
            "the guard must not touch files"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_only_output_does_not_leak_the_rule_or_the_path() {
        let dir = temp_dir("quiet");
        fs::write(dir.join("secret-name.md"), b"contains SYNTHETIC_TOKEN_9996").expect("write");
        let rules = RuleSet {
            rules: vec![Rule {
                id: "r1".to_owned(),
                kind: Kind::Content,
                needle: b"SYNTHETIC_TOKEN_9996".to_vec(),
            }],
        };
        let report = scan(
            &["secret-name.md".to_owned()],
            &rules,
            Some(&dir),
            &HashSet::new(),
        );
        let mut buffer = Vec::new();
        print_findings(&report, false, &mut buffer);
        let text = String::from_utf8(buffer).expect("utf8");
        assert!(
            !text.contains("secret-name"),
            "path must stay hidden: {text}"
        );
        assert!(
            !text.contains("SYNTHETIC_TOKEN_9996"),
            "rule must stay hidden: {text}"
        );
        assert!(text.contains("PROHIBITED rule=r1 path-sha256="), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manual_clone_url_and_the_ci_url_name_the_same_origin() {
        let expected = normalize_origin("https://github.com/nguyennhat2002/CutCraft.git");
        for actual in [
            "https://github.com/nguyennhat2002/CutCraft",
            "https://github.com/nguyennhat2002/CutCraft.git",
            "https://github.com/nguyennhat2002/CutCraft.git/",
        ] {
            assert_eq!(normalize_origin(actual), expected, "{actual}");
        }
    }

    #[test]
    fn a_short_or_foreign_origin_never_matches_the_full_url() {
        let expected = normalize_origin("https://github.com/nguyennhat2002/CutCraft.git");
        for actual in [
            "CutCraft",
            "nguyennhat2002/CutCraft",
            "https://github.com/someone-else/CutCraft.git",
            "https://gitlab.com/nguyennhat2002/CutCraft.git",
        ] {
            assert_ne!(
                normalize_origin(actual),
                expected,
                "{actual} must fail closed"
            );
        }
    }

    #[test]
    fn a_clean_tree_produces_no_findings() {
        let dir = temp_dir("clean");
        fs::write(dir.join("lib.rs"), b"fn main() {}").expect("write");
        let rules = RuleSet {
            rules: vec![Rule {
                id: "test/path".to_owned(),
                kind: Kind::PathDigest,
                needle: path_digest("forbidden").into_bytes(),
            }],
        };
        let report = scan(&["lib.rs".to_owned()], &rules, Some(&dir), &HashSet::new());
        assert!(report.findings.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
