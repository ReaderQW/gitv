use std::path::Path;

use git2::{Repository, TreeEntry, TreeWalkMode, TreeWalkResult};

use crate::error::CoreResult;
use crate::model::SnapshotRecord;

/// Scan the HEAD tree of a repository and return snapshot records for each file,
/// counting blank lines, comment lines, and code lines.
pub fn scan_snapshot(repo: &Repository) -> CoreResult<Vec<SnapshotRecord>> {
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;
    let mut snapshots = Vec::new();

    let mut cb = |root: &str, entry: &TreeEntry| -> TreeWalkResult {
        let name = match entry.name() {
            Some(n) => n,
            None => return TreeWalkResult::Ok,
        };

        // Skip submodules (which have no blob content)
        if entry.kind() != Some(git2::ObjectType::Blob) {
            return TreeWalkResult::Ok;
        }

        let file_path = if root.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", root, name)
        };

        // Try to read file content as text; skip binary files
        let blob = match entry.to_object(repo).and_then(|o| o.peel_to_blob()) {
            Ok(b) => b,
            Err(_) => return TreeWalkResult::Ok,
        };

        let content = match std::str::from_utf8(blob.content()) {
            Ok(s) => s,
            Err(_) => return TreeWalkResult::Ok, // skip binary
        };

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len() as u32;

        let ext = Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let blank = lines.iter().filter(|l| l.trim().is_empty()).count() as u32;
        let comment = count_comment_lines(&lines, &ext);
        let code = total.saturating_sub(blank + comment);

        snapshots.push(SnapshotRecord {
            commit_hash: String::new(), // HEAD snapshot, not tied to a specific commit
            file_path,
            code_lines: code,
            comment_lines: comment,
            blank_lines: blank,
        });

        TreeWalkResult::Ok
    };

    tree.walk(TreeWalkMode::PreOrder, &mut cb)?;

    Ok(snapshots)
}

/// Count lines that consist entirely of comments (single-line or multi-line).
fn count_comment_lines(lines: &[&str], ext: &str) -> u32 {
    let single_markers = single_line_comment_markers(ext);
    let (multi_start, multi_end) = multi_line_comment_markers(ext);

    let mut count = 0u32;
    let mut in_block = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Inside a multi-line block comment
        if in_block {
            count += 1;
            if !multi_end.is_empty() && trimmed.contains(multi_end) {
                in_block = false;
            }
            continue;
        }

        // Check for multi-line comment start
        if !multi_start.is_empty() {
            if let Some(pos) = trimmed.find(multi_start) {
                // Only count if the marker is not inside a string literal
                let before = &trimmed[..pos];
                let in_string = before.matches('"').count() % 2 == 1;
                if !in_string {
                    count += 1;
                    let after = &trimmed[pos + multi_start.len()..];
                    if multi_end.is_empty() || !after.contains(multi_end) {
                        in_block = true;
                    }
                    continue;
                }
            }
        }

        // Check for single-line comment markers (line starts with marker)
        for marker in &single_markers {
            if trimmed.starts_with(marker) {
                count += 1;
                break;
            }
        }
    }

    count
}

/// Return single-line comment markers for a given file extension.
fn single_line_comment_markers(ext: &str) -> Vec<&'static str> {
    match ext {
        // C-like: // and also #
        "rs" | "go" | "zig" => vec!["//"],
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx" | "java" | "js"
        | "jsx" | "mjs" | "ts" | "tsx" | "swift" | "kt" | "kts" | "scala"
        | "dart" | "cs" => vec!["//"],
        // Hash-based
        "py" | "pyw" | "rb" | "sh" | "bash" | "zsh" | "pl" | "pm" | "r"
        | "rake" | "gemspec" | "yaml" | "yml" | "toml" | "ini" | "cfg"
        | "conf" | "makefile" | "dockerfile" | "gitignore" | "env"
        | "nu" | "nuspec" | "ps1" => vec!["#"],
        // SQL / Lua / Haskell
        "sql" | "lua" | "hs" | "sqlite" | "ada" => vec!["--"],
        // Lisp / Clojure
        "lisp" | "clj" | "cljs" | "cljc" | "edn" | "el" => vec![";"],
        // TeX
        "tex" | "sty" | "cls" | "bib" => vec!["%"],
        // OCaml
        "ml" | "mli" | "mll" | "mly" => vec!["(*"],
        // Batch
        "bat" | "cmd" => vec!["rem ", "REM "],
        // Default to hash-based
        _ => vec!["#"],
    }
}

/// Return (start_marker, end_marker) for multi-line comments, or ("", "") if none.
fn multi_line_comment_markers(ext: &str) -> (&'static str, &'static str) {
    match ext {
        "rs" | "go" | "zig" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx"
        | "java" | "js" | "jsx" | "mjs" | "ts" | "tsx" | "swift" | "kt" | "kts"
        | "scala" | "dart" | "cs" | "css" | "scss" | "sass" | "less" | "php"
        | "php3" | "php4" | "php5" | "phtml" | "sql" | "rust" | "graphql" => ("/*", "*/"),
        "html" | "htm" | "xhtml" | "xml" | "xsd" | "xslt" | "svg" | "mdx" => ("<!--", "-->"),
        "ml" | "mli" | "mll" | "mly" => ("(*", "*)"),
        _ => ("", ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree_entry_snapshot(lines: &[&str], ext: &str) -> SnapshotRecord {
        let blank = lines.iter().filter(|l| l.trim().is_empty()).count() as u32;
        let comment = count_comment_lines(lines, ext);
        let code = lines.len() as u32 - blank - comment;
        SnapshotRecord {
            commit_hash: String::new(),
            file_path: format!("file.{}", ext),
            code_lines: code,
            comment_lines: comment,
            blank_lines: blank,
        }
    }

    #[test]
    fn test_rust_no_comments() {
        let lines = vec![
            "fn main() {",
            "    println!(\"hello\");",
            "}",
        ];
        let snap = make_tree_entry_snapshot(&lines, "rs");
        assert_eq!(snap.code_lines, 3);
        assert_eq!(snap.comment_lines, 0);
        assert_eq!(snap.blank_lines, 0);
    }

    #[test]
    fn test_rust_with_comments() {
        let lines = vec![
            "//! Module doc comment",
            "",
            "fn main() {",
            "    // inline comment",
            "    println!(\"hello\");",
            "}",
        ];
        let snap = make_tree_entry_snapshot(&lines, "rs");
        assert_eq!(snap.blank_lines, 1);
        assert_eq!(snap.comment_lines, 2);
        assert_eq!(snap.code_lines, 3);
    }

    #[test]
    fn test_python_with_comments() {
        let lines = vec![
            "# This is a comment",
            "def hello():",
            "    # another comment",
            "    print('hi')",
            "",
        ];
        let snap = make_tree_entry_snapshot(&lines, "py");
        assert_eq!(snap.blank_lines, 1);
        assert_eq!(snap.comment_lines, 2);
        assert_eq!(snap.code_lines, 2);
    }

    #[test]
    fn test_multi_line_block_comment() {
        let lines = vec![
            "/*",
            " * This is a block comment",
            " * spanning multiple lines",
            " */",
            "fn main() {}",
        ];
        let snap = make_tree_entry_snapshot(&lines, "rs");
        assert_eq!(snap.comment_lines, 4);
        assert_eq!(snap.code_lines, 1);
        assert_eq!(snap.blank_lines, 0);
    }

    #[test]
    fn test_html_comments() {
        let lines = vec![
            "<!--",
            "  HTML comment block",
            "-->",
            "<html></html>",
        ];
        let snap = make_tree_entry_snapshot(&lines, "html");
        assert_eq!(snap.comment_lines, 3);
        assert_eq!(snap.code_lines, 1);
    }

    #[test]
    fn test_empty_file() {
        let lines: Vec<&str> = vec![];
        let snap = make_tree_entry_snapshot(&lines, "rs");
        assert_eq!(snap.code_lines, 0);
        assert_eq!(snap.comment_lines, 0);
        assert_eq!(snap.blank_lines, 0);
    }

    #[test]
    fn test_all_blank() {
        let lines = vec!["", "  ", "   ", ""];
        let snap = make_tree_entry_snapshot(&lines, "rs");
        assert_eq!(snap.code_lines, 0);
        assert_eq!(snap.comment_lines, 0);
        assert_eq!(snap.blank_lines, 4);
    }

    #[test]
    fn test_snapshot_empty_repo() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let repo_path = dir.path().join("empty");
        let repo = git2::Repository::init(&repo_path).expect("init");
        // No commits yet — head() should fail
        let result = scan_snapshot(&repo);
        assert!(result.is_err(), "empty repo should error");
    }

    #[test]
    fn test_snapshot_with_initial_commit() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let repo_path = dir.path().join("test_snap");
        let repo = git2::Repository::init(&repo_path).expect("init");

        let sig = git2::Signature::now("Test", "test@test.com").expect("sig");

        // Create a Rust file
        std::fs::write(repo_path.join("main.rs"), b"// comment\nfn main() {\n    println!(\"hi\");\n}\n").expect("write");
        // Create a Python file
        std::fs::write(repo_path.join("lib.py"), b"# py comment\ndef foo():\n    pass\n").expect("write");
        // Create an empty file
        std::fs::write(repo_path.join("empty.md"), b"").expect("write");

        let mut index = repo.index().expect("index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[]).expect("commit");

        let snapshots = scan_snapshot(&repo).expect("scan snapshot");
        assert_eq!(snapshots.len(), 3, "should have 3 files");

        // Verify main.rs
        let main_rs = snapshots.iter().find(|s| s.file_path == "main.rs").expect("main.rs");
        assert_eq!(main_rs.comment_lines, 1);
        assert_eq!(main_rs.code_lines, 3);

        // Verify lib.py
        let lib_py = snapshots.iter().find(|s| s.file_path == "lib.py").expect("lib.py");
        assert_eq!(lib_py.comment_lines, 1);
        assert_eq!(lib_py.code_lines, 2);
        assert_eq!(lib_py.blank_lines, 0);

        // Verify empty.md
        let empty_md = snapshots.iter().find(|s| s.file_path == "empty.md").expect("empty.md");
        assert_eq!(empty_md.code_lines, 0);
        assert_eq!(empty_md.comment_lines, 0);
        assert_eq!(empty_md.blank_lines, 0);
    }
}
