// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Boundary-condition tests for core modules (hash / scanner / sync / diff).
//! All filesystem tests run inside tempfile::tempdir() — no user data is touched.

#![cfg(test)]

use crate::db::Database;
use crate::diff;
use crate::hash;
use crate::scanner;
use crate::sync;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

// ==================== hash::compute_id_hash ====================

#[test]
fn id_hash_empty_string() {
    // Boundary: empty input must not panic and must stay in JS safe-integer range
    let h = hash::compute_id_hash("");
    assert!(h >= 0);
    assert!(h <= (1i64 << 53) - 1);
}

#[test]
fn id_hash_unicode_and_long_input() {
    let h1 = hash::compute_id_hash("技能管理器🚀");
    let h2 = hash::compute_id_hash(&"x".repeat(1_000_000));
    for h in [h1, h2] {
        assert!(h >= 0 && h <= (1i64 << 53) - 1);
    }
    // Deterministic
    assert_eq!(h1, hash::compute_id_hash("技能管理器🚀"));
}

// ==================== hash::compute_content_hash ====================

#[test]
fn content_hash_nonexistent_dir_is_err() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("no-such-dir");
    assert!(hash::compute_content_hash(&missing).is_err());
}

#[test]
fn content_hash_empty_dir_ok() {
    let dir = tempdir().unwrap();
    let h = hash::compute_content_hash(dir.path()).unwrap();
    // SHA-256 of empty input
    assert_eq!(h, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn content_hash_ignores_hidden_and_local_md() {
    // Boundary: dir containing ONLY hidden files + local.md must equal empty-dir hash
    let empty = tempdir().unwrap();
    let noisy = tempdir().unwrap();
    write_file(&noisy.path().join(".hidden"), "secret");
    write_file(&noisy.path().join("local.md"), "# marker");
    let h_empty = hash::compute_content_hash(empty.path()).unwrap();
    let h_noisy = hash::compute_content_hash(noisy.path()).unwrap();
    assert_eq!(h_empty, h_noisy);
}

#[test]
fn content_hash_is_location_independent() {
    // Same relative structure in two different temp dirs → same hash
    let a = tempdir().unwrap();
    let b = tempdir().unwrap();
    for d in [a.path(), b.path()] {
        write_file(&d.join("SKILL.md"), "content");
        write_file(&d.join("sub").join("x.txt"), "nested");
    }
    assert_eq!(
        hash::compute_content_hash(a.path()).unwrap(),
        hash::compute_content_hash(b.path()).unwrap()
    );
}

#[test]
fn content_hash_sensitive_to_rename() {
    let a = tempdir().unwrap();
    let b = tempdir().unwrap();
    write_file(&a.path().join("a.txt"), "same");
    write_file(&b.path().join("b.txt"), "same");
    assert_ne!(
        hash::compute_content_hash(a.path()).unwrap(),
        hash::compute_content_hash(b.path()).unwrap()
    );
}

// ==================== hash::compute_core_hash ====================

#[test]
fn core_hash_missing_file_is_err() {
    let dir = tempdir().unwrap();
    assert!(hash::compute_core_hash(&dir.path().join("SKILL.md")).is_err());
}

#[test]
fn core_hash_empty_file_ok() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("SKILL.md");
    write_file(&f, "");
    let h = hash::compute_core_hash(&f).unwrap();
    assert_eq!(h, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

// ==================== scanner::scan_directory ====================

#[test]
fn scan_nonexistent_path_is_err() {
    let dir = tempdir().unwrap();
    assert!(scanner::scan_directory(&dir.path().join("nope")).is_err());
}

#[test]
fn scan_path_is_file_not_dir_is_err() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("file.txt");
    write_file(&f, "x");
    assert!(scanner::scan_directory(&f).is_err());
}

#[test]
fn scan_empty_dir_returns_empty_ok() {
    let dir = tempdir().unwrap();
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert!(skills.is_empty());
}

#[test]
fn scan_skill_md_without_front_matter_uses_dir_name() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("my-skill");
    write_file(&skill_dir.join("SKILL.md"), "Just plain markdown, no front matter.");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "my-skill");
    assert_eq!(skills[0].description, None);
}

#[test]
fn scan_unclosed_front_matter_falls_back_to_dir_name() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("broken-fm");
    write_file(&skill_dir.join("SKILL.md"), "---\nname: never-closed\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "broken-fm");
}

#[test]
fn scan_empty_front_matter_falls_back_to_dir_name() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("empty-fm");
    write_file(&skill_dir.join("SKILL.md"), "---\n---\nbody");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "empty-fm");
}

#[test]
fn scan_invalid_yaml_is_err_when_only_skill() {
    // Malformed YAML → parse error; dir has no other skill → Err with message
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("bad-yaml");
    write_file(&skill_dir.join("SKILL.md"), "---\nname: [unclosed\n---\n");
    let result = scanner::scan_directory(dir.path());
    assert!(result.is_err());
}

#[test]
fn scan_invalid_yaml_does_not_block_sibling_skills() {
    // One bad skill + one good skill → good one still returned (errors swallowed)
    let dir = tempdir().unwrap();
    write_file(&dir.path().join("bad").join("SKILL.md"), "---\nname: [unclosed\n---\n");
    write_file(&dir.path().join("good").join("SKILL.md"), "---\nname: good-skill\n---\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "good-skill");
}

#[test]
fn scan_non_string_yaml_name_falls_back_to_dir_name() {
    // Boundary: name: 123 (number) → as_str() = None → dir name
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("numeric-name");
    write_file(&skill_dir.join("SKILL.md"), "---\nname: 123\n---\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills[0].name, "numeric-name");
}

#[test]
fn scan_unicode_name_and_description() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("cn");
    write_file(
        &skill_dir.join("SKILL.md"),
        "---\nname: 中文技能名\ndescription: 支持 emoji 🚀\n---\n",
    );
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills[0].name, "中文技能名");
    assert_eq!(skills[0].description.as_deref(), Some("支持 emoji 🚀"));
}

#[test]
fn scan_stops_recursion_at_skill_md() {
    // Nested SKILL.md under a skill dir must NOT be discovered
    let dir = tempdir().unwrap();
    let outer = dir.path().join("outer");
    write_file(&outer.join("SKILL.md"), "---\nname: outer\n---\n");
    write_file(&outer.join("inner").join("SKILL.md"), "---\nname: inner\n---\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "outer");
}

#[test]
fn scan_skips_hidden_directories() {
    let dir = tempdir().unwrap();
    write_file(&dir.path().join(".hidden").join("s").join("SKILL.md"), "---\nname: ghost\n---\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert!(skills.is_empty());
}

#[test]
fn scan_deeply_nested_skill_found() {
    let dir = tempdir().unwrap();
    let deep = dir.path().join("a").join("b").join("c").join("d").join("skill");
    write_file(&deep.join("SKILL.md"), "---\nname: deep\n---\n");
    let skills = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "deep");
}

// ==================== scanner::expand_path / normalize_path ====================

#[test]
fn expand_path_tilde_variants() {
    let home = dirs::home_dir().unwrap();
    assert_eq!(scanner::expand_path("~").unwrap(), home);
    assert_eq!(scanner::expand_path("~/x").unwrap(), home.join("x"));
    assert_eq!(scanner::expand_path("~\\x").unwrap(), home.join("x"));
}

#[test]
fn expand_path_empty_string_yields_empty_path() {
    // Boundary: "" is passed through as-is (documents current behavior)
    let p = scanner::expand_path("").unwrap();
    assert_eq!(p, std::path::PathBuf::from(""));
}

#[test]
fn expand_path_tilde_username_not_expanded() {
    // "~user/x" style is NOT expanded — stays literal
    let p = scanner::expand_path("~user/x").unwrap();
    assert_eq!(p, std::path::PathBuf::from("~user/x"));
}

#[test]
#[cfg(target_os = "windows")]
fn normalize_path_strips_verbatim_prefix_and_unifies_slashes() {
    let p = std::path::PathBuf::from(r"\\?\C:\Users/test\dir");
    assert_eq!(scanner::normalize_path(&p), r"C:\Users\test\dir");
}

// ==================== sync::copy_directory / replace_directory ====================

#[test]
fn copy_directory_nonexistent_src_is_err() {
    let dir = tempdir().unwrap();
    let err = sync::copy_directory(&dir.path().join("missing"), &dir.path().join("dst"));
    assert!(err.is_err());
}

#[test]
fn copy_directory_src_is_file_is_err() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("f.txt");
    write_file(&f, "x");
    assert!(sync::copy_directory(&f, &dir.path().join("dst")).is_err());
}

#[test]
fn copy_directory_empty_src_creates_empty_dst() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let dst = dir.path().join("dst");
    sync::copy_directory(&src, &dst).unwrap();
    assert!(dst.is_dir());
    assert_eq!(fs::read_dir(&dst).unwrap().count(), 0);
}

#[test]
fn copy_directory_skips_hidden_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    write_file(&src.join("visible.md"), "v");
    write_file(&src.join(".secret"), "s");
    let dst = dir.path().join("dst");
    sync::copy_directory(&src, &dst).unwrap();
    assert!(dst.join("visible.md").exists());
    assert!(!dst.join(".secret").exists());
}

#[test]
fn replace_directory_removes_stale_files() {
    // Boundary: dst has an extra file that src doesn't — replace must delete it
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    write_file(&src.join("keep.md"), "new");
    write_file(&dst.join("keep.md"), "old");
    write_file(&dst.join("stale.md"), "should disappear");
    sync::replace_directory(&src, &dst).unwrap();
    assert_eq!(fs::read_to_string(dst.join("keep.md")).unwrap(), "new");
    assert!(!dst.join("stale.md").exists());
}

#[test]
fn replace_directory_creates_missing_parents() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    write_file(&src.join("a.md"), "a");
    let dst = dir.path().join("x").join("y").join("z");
    sync::replace_directory(&src, &dst).unwrap();
    assert!(dst.join("a.md").exists());
}

// ==================== sync::symlink_or_copy ====================

#[test]
fn symlink_or_copy_missing_target_is_err() {
    let dir = tempdir().unwrap();
    let r = sync::symlink_or_copy(&dir.path().join("missing"), &dir.path().join("link"));
    assert!(r.is_err());
}

#[test]
fn symlink_or_copy_replaces_existing_dst() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("target");
    write_file(&target.join("new.md"), "new");
    let link = dir.path().join("link");
    write_file(&link.join("old.md"), "old");
    let method = sync::symlink_or_copy(&target, &link).unwrap();
    // Windows without dev-mode falls back to copy; both outcomes acceptable
    assert!(method == "symlink" || method == "copy");
    assert!(link.join("new.md").exists());
    assert!(!link.join("old.md").exists());
}

// ==================== sync::ssot_path / create_local_marker ====================

#[test]
fn ssot_path_global_vs_project_layout() {
    let g = sync::ssot_path("skill-a", 0).unwrap();
    let p = sync::ssot_path("skill-a", 42).unwrap();
    assert!(g.ends_with(Path::new("local").join("skill-a")));
    assert!(p.ends_with(Path::new("local").join("_p42").join("skill-a")));
}

#[test]
fn ssot_path_rejects_traversal_and_separator_names() {
    // Security fix: skill names come from YAML front matter (untrusted input).
    // Names that could escape ~/.agents/skills/local must be rejected.
    for bad in ["../evil", "..", ".", "a/b", "a\\b", "", "   "] {
        assert!(
            sync::ssot_path(bad, 0).is_err(),
            "name {:?} should be rejected",
            bad
        );
    }
    // Normal names still work
    assert!(sync::ssot_path("my-skill", 0).is_ok());
    assert!(sync::ssot_path("中文技能名", 7).is_ok());
}

#[test]
fn create_local_marker_in_missing_dir_is_err() {
    let dir = tempdir().unwrap();
    let r = sync::create_local_marker(&dir.path().join("missing"));
    assert!(r.is_err());
}

#[test]
fn create_local_marker_does_not_overwrite_existing() {
    let dir = tempdir().unwrap();
    let marker = dir.path().join("local.md");
    write_file(&marker, "user notes");
    sync::create_local_marker(dir.path()).unwrap();
    assert_eq!(fs::read_to_string(&marker).unwrap(), "user notes");
}

// ==================== diff::compute_skill_diff ====================

#[test]
fn diff_both_dirs_missing_no_changes() {
    let dir = tempdir().unwrap();
    let d = diff::compute_skill_diff(
        &dir.path().join("nope1"),
        &dir.path().join("nope2"),
        "ghost",
    )
    .unwrap();
    assert!(!d.has_changes);
    assert!(d.files.is_empty());
}

#[test]
fn diff_identical_dirs_no_changes() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    for d in [&a, &b] {
        write_file(&d.join("SKILL.md"), "same\ncontent\n");
    }
    // local.md must be ignored even when it differs
    write_file(&a.join("local.md"), "A");
    write_file(&b.join("local.md"), "B");
    let d = diff::compute_skill_diff(&a, &b, "s").unwrap();
    assert!(!d.has_changes);
}

#[test]
fn diff_added_empty_file_has_no_hunks() {
    // Boundary: 0-byte new file → Added entry with empty hunks
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    fs::create_dir_all(&ssot).unwrap();
    write_file(&src.join("empty.md"), "");
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes);
    assert_eq!(d.files.len(), 1);
    assert!(d.files[0].hunks.is_empty());
}

#[test]
fn diff_deleted_file_detected() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    fs::create_dir_all(&src).unwrap();
    write_file(&ssot.join("gone.md"), "line1\nline2\n");
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert_eq!(d.files.len(), 1);
    assert!(matches!(d.files[0].change, diff::FileChange::Deleted));
    assert_eq!(d.files[0].hunks[0].lines.len(), 2);
}

#[test]
fn diff_large_file_uses_full_replace_fallback() {
    // Boundary: >5000 combined lines triggers the non-LCS path (no OOM / hang)
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    let big_old: String = (0..3000).map(|i| format!("old line {}\n", i)).collect();
    let big_new: String = (0..3000).map(|i| format!("new line {}\n", i)).collect();
    write_file(&ssot.join("big.md"), &big_old);
    write_file(&src.join("big.md"), &big_new);
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes);
    assert_eq!(d.files[0].hunks.len(), 1);
    assert_eq!(d.files[0].hunks[0].old_count, 3000);
    assert_eq!(d.files[0].hunks[0].new_count, 3000);
}

#[test]
fn diff_crlf_vs_lf_shows_format_only_hunk() {
    // Fixed: bytes differ (CRLF vs LF) but lines are identical — instead of a
    // blank "modified" entry, an informational format hunk is emitted.
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    write_file(&src.join("f.md"), "a\r\nb\r\n");
    write_file(&ssot.join("f.md"), "a\nb\n");
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes);
    assert!(matches!(d.files[0].change, diff::FileChange::Modified));
    let hunk = &d.files[0].hunks[0];
    assert!(hunk.lines.iter().any(|l| l.op == "-" && l.content.contains("LF")));
    assert!(hunk.lines.iter().any(|l| l.op == "+" && l.content.contains("CRLF")));
}

#[test]
fn diff_trailing_newline_only_shows_format_only_hunk() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    write_file(&src.join("f.md"), "a\nb"); // no trailing newline
    write_file(&ssot.join("f.md"), "a\nb\n");
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes);
    let hunk = &d.files[0].hunks[0];
    assert!(hunk.lines.iter().any(|l| l.op == "-" && l.content.contains("with trailing newline")));
    assert!(hunk.lines.iter().any(|l| l.op == "+" && l.content.contains("no trailing newline")));
}

#[test]
fn diff_differing_binary_files_detected() {
    // Fixed: raw-byte comparison — two DIFFERENT binaries must be reported as
    // Modified with a size-only placeholder hunk (no lossy text collapse).
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&ssot).unwrap();
    fs::write(src.join("img.bin"), [0xFFu8, 0xFE, 0x01]).unwrap();
    fs::write(ssot.join("img.bin"), [0xFFu8, 0xFE, 0x99, 0x77]).unwrap();
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes, "differing binary files must be detected");
    assert!(matches!(d.files[0].change, diff::FileChange::Modified));
    let hunk = &d.files[0].hunks[0];
    assert!(hunk.lines.iter().any(|l| l.op == "-" && l.content.contains("4 bytes")));
    assert!(hunk.lines.iter().any(|l| l.op == "+" && l.content.contains("3 bytes")));
}

#[test]
fn diff_identical_binary_files_no_changes() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&ssot).unwrap();
    fs::write(src.join("img.bin"), [0xFFu8, 0xFE, 0x01]).unwrap();
    fs::write(ssot.join("img.bin"), [0xFFu8, 0xFE, 0x01]).unwrap();
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(!d.has_changes);
}

#[test]
fn diff_added_binary_file_has_placeholder_hunk() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    fs::create_dir_all(&ssot).unwrap();
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("new.bin"), [0x00u8, 0xFF, 0xFE]).unwrap();
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(matches!(d.files[0].change, diff::FileChange::Added));
    let hunk = &d.files[0].hunks[0];
    assert_eq!(hunk.lines.len(), 1);
    assert!(hunk.lines[0].content.contains("binary file"));
}

#[test]
fn diff_unicode_content_modified() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let ssot = dir.path().join("ssot");
    write_file(&src.join("f.md"), "你好\n世界 v2\n");
    write_file(&ssot.join("f.md"), "你好\n世界\n");
    let d = diff::compute_skill_diff(&src, &ssot, "s").unwrap();
    assert!(d.has_changes);
    let hunk = &d.files[0].hunks[0];
    assert!(hunk.lines.iter().any(|l| l.op == "-" && l.content == "世界"));
    assert!(hunk.lines.iter().any(|l| l.op == "+" && l.content == "世界 v2"));
}

// ==================== end-to-end boundary: scan → hash 稳定性 ====================

#[test]
fn scan_then_copy_preserves_core_hash_but_content_hash_gains_marker() {
    // Simulates sync flow: copy skill dir + add local.md marker.
    // core_hash (SKILL.md only) must stay identical; content_hash must ignore local.md.
    let dir = tempdir().unwrap();
    let src = dir.path().join("skills").join("demo");
    write_file(&src.join("SKILL.md"), "---\nname: demo\n---\nbody");
    write_file(&src.join("ref.txt"), "ref");

    let dst = dir.path().join("ssot").join("demo");
    sync::copy_directory(&src, &dst).unwrap();
    sync::create_local_marker(&dst).unwrap();

    let src_core = hash::compute_core_hash(&src.join("SKILL.md")).unwrap();
    let dst_core = hash::compute_core_hash(&dst.join("SKILL.md")).unwrap();
    assert_eq!(src_core, dst_core);

    let src_content = hash::compute_content_hash(&src).unwrap();
    let dst_content = hash::compute_content_hash(&dst).unwrap();
    assert_eq!(src_content, dst_content, "local.md must not affect content hash");
}

// ==================== db::Database (in-memory, never touches user data) ====================

#[test]
fn db_seed_data_populates_preset_tools_and_global_project() {
    let db = Database::new_in_memory().unwrap();
    let tools = db.list_tools().unwrap();
    assert_eq!(tools.len(), 5, "expected 5 preset tools");
    assert!(tools.iter().any(|t| t.name == "Claude Code"));
    // Global project (id=0) must exist so project_id=0 FKs are valid
    assert_eq!(db.get_project_name(0).unwrap(), "Global");
}

#[test]
fn db_upsert_skill_name_as_identity() {
    let db = Database::new_in_memory().unwrap();

    // First upsert creates the record
    let (id1, is_new1) = db
        .upsert_skill("my-skill", Some("v1"), "/path/a", "hash-a", "core-a", 0)
        .unwrap();
    assert!(is_new1);

    // Same (name, project) from a different tool path → same record, updated in place
    let (id2, is_new2) = db
        .upsert_skill("my-skill", Some("v2"), "/path/b", "hash-b", "core-b", 0)
        .unwrap();
    assert_eq!(id1, id2, "same name+project must merge into one record");
    assert!(!is_new2);

    let skill = db.get_skill_by_id(id1).unwrap();
    assert_eq!(skill.content_hash, "hash-b", "upsert must refresh hashes");
    assert_eq!(skill.source_path, "/path/b");

    // Same name in a different project scope → separate record
    let project = db.add_project("proj", "/tmp/proj").unwrap();
    let (id3, is_new3) = db
        .upsert_skill("my-skill", None, "/path/c", "hash-c", "core-c", project.id)
        .unwrap();
    assert!(is_new3);
    assert_ne!(id1, id3, "different project must not share the skill record");
}

#[test]
fn db_conflict_lifecycle() {
    let db = Database::new_in_memory().unwrap();
    let (skill_id, _) = db
        .upsert_skill("conflicted", None, "/p", "h", "c", 0)
        .unwrap();

    assert!(!db.has_unresolved_conflict(skill_id).unwrap());

    let detail = r#"[{"tool_id":1,"tool_name":"Claude Code","core_hash":"aaa","source_path":"/x"},
                     {"tool_id":2,"tool_name":"Codex CLI","core_hash":"bbb","source_path":"/y"}]"#;
    let conflict_id = db.insert_conflict(skill_id, detail).unwrap();

    assert!(db.has_unresolved_conflict(skill_id).unwrap());
    let list = db.list_unresolved_conflicts(0).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].skill_name, "conflicted");
    assert_eq!(list[0].versions.len(), 2, "detail JSON must parse into versions");

    db.resolve_conflict_record(conflict_id, "keep_ssot").unwrap();
    assert!(!db.has_unresolved_conflict(skill_id).unwrap());
    assert!(db.list_unresolved_conflicts(0).unwrap().is_empty());
}

#[test]
fn db_conflict_with_invalid_detail_json_does_not_break_listing() {
    // Boundary: corrupted detail must degrade to empty versions, not fail the query
    let db = Database::new_in_memory().unwrap();
    let (skill_id, _) = db.upsert_skill("bad-json", None, "/p", "h", "c", 0).unwrap();
    db.insert_conflict(skill_id, "not valid json {{{").unwrap();

    let list = db.list_unresolved_conflicts(0).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].versions.is_empty());
}

#[test]
fn db_ensure_installation_preserves_explicit_disable() {
    let db = Database::new_in_memory().unwrap();
    let tool_id = db.list_tools().unwrap()[0].id;
    let (skill_id, _) = db.upsert_skill("inst", None, "/p", "h", "c", 0).unwrap();

    // Fresh scan → auto-detected as active
    db.ensure_installation(skill_id, tool_id, 0).unwrap();
    assert_eq!(db.get_active_installations(skill_id, 0).unwrap().len(), 1);

    // User explicitly disables
    db.toggle_installation(skill_id, tool_id, 0, false).unwrap();
    assert!(db.get_active_installations(skill_id, 0).unwrap().is_empty());

    // Re-scan must NOT override the explicit disable
    db.ensure_installation(skill_id, tool_id, 0).unwrap();
    assert!(
        db.get_active_installations(skill_id, 0).unwrap().is_empty(),
        "ensure_installation must not re-activate a user-disabled installation"
    );

    // Explicit re-enable works
    db.toggle_installation(skill_id, tool_id, 0, true).unwrap();
    assert_eq!(db.get_active_installations(skill_id, 0).unwrap().len(), 1);
}
