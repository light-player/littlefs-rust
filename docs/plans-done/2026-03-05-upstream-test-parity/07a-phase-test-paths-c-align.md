# Phase 7a: test_paths C-Alignment

## Scope

Align the 5 existing test_paths cases with the C reference. Tests are already implemented but simplified; expand them to match C exactly.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_paths.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

`reference/tests/test_paths.toml`

## Cases

### test_paths_noent_trailing_dotdots

C has: INVAL above root, ISDIR for file_open on `coffee/_rip/..`, dir_open success for `coffee/_rip/..`, rename (bad source/dest, valid rename `coffee/thai_/..` → `espresso/mocha`), remove (NOTEMPTY, INVAL). Add all C cases.

### test_paths_utf8_ipa

C adds: file_open WRONLY|CREAT => ISDIR (dir mode) or success (file mode); file_open WRONLY|CREAT|EXCL => EXIST. Add these checks.

### test_paths_oopsallspaces

C layout: root `" "`, children `" / "`, `" /  "`, `" /   "`, `" /    "`, `" /     "`, `" /      "` (6 children). Stat all, file_open/dir_open matrix, rename to `"  /      "` etc., remove. Match C exactly.

### test_paths_oopsalldels

C layout: root `\x7f` (1 byte), children `\x7f/\x7f`, `\x7f/\x7f\x7f`, … (6 children with 1–6 DEL bytes). Use `path_bytes_raw` for non-UTF8 paths. Stat, file_open, dir_open, rename to `\x7f\x7f/…`, remove. Match C exactly.

### test_paths_oopsallffs

Same as oopsalldels but with `0xff` bytes. Match C exactly.
