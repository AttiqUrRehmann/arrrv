# Findings

## Files with `arrrv` references

### Rust Source
- `src/cache.rs:6` — `.join("arrrv")` (cache path segment)
- `src/cache.rs:20` — doc comment mentioning `.arrrv/library/`
- `src/cache.rs:51` — test assertion: `ends_with("arrrv/packages/ggplot2_3.5.1")`
- `src/config.rs:6` — `pub const CONFIG_FILE: &str = "arrrv.toml";`
- `src/config.rs:26,54` — error strings: `run \`arrrv init\` first`
- `src/config.rs:364` — doc comment referencing `arrrv.toml`
- `src/config.rs:390` — expect message: `"failed to parse arrrv.toml"`
- `src/lockfile.rs:9` — doc comment
- `src/lockfile.rs:17` — `Path::new("arrrv.lock")` (functional!)
- `src/lockfile.rs:18` — `println!("wrote arrrv.lock")`
- `src/lockfile.rs:30` — generated file header string
- `src/lockfile.rs:85` — doc comment
- `src/lockfile.rs:87,94` — `read_to_string("arrrv.lock")` (functional!)
- `src/lockfile.rs:88` — expect message with `arrrv lock`
- `src/lockfile.rs:138` — expect message: `"failed to parse arrrv.lock"`
- `src/main.rs:21` — `const LIB_DIR: &str = ".arrrv/library";`
- `src/main.rs:24` — `#[command(name = "arrrv", ...)]`
- `src/main.rs:35,42,44,46` — doc comments on CLI variants
- `src/main.rs:82,83,164,206,207,210,211,218` — user-facing strings
- `src/resolver.rs:166,340,495` — doc/test comments only

### Infrastructure
- `scripts/install.sh` — binary name, URLs, tarball filenames (~12 occurrences)
- `formula/arrrv.rb` — entire file; needs `git mv` to `formula/ruv.rb`
- `.github/workflows/release.yml` — tarball packaging steps (need to verify line numbers)

### Config Artifacts
- `arrrv.toml` — repo root, needs `git mv` to `ruv.toml`
- `arrrv.lock` — repo root, needs `git mv` to `ruv.lock`
- `.gitignore` — 3 entries: `arrrv_lib`, `.arrrv`, `arrrv.lock`

### Documentation
- `README.md` — ~30 occurrences
- `DISTRIBUTION.md` — ~25 occurrences
- `mvp_plan.md` — ~15 occurrences
- `project_plan.md` — ~30 occurrences
- `test_coverage.md` — 2 occurrences

## Critical constraint
Tarball structure is a 3-way contract: `release.yml` → `install.sh` → `formula/ruv.rb`
All three must agree on: `ruv-$TARGET.tar.gz` containing `ruv-$TARGET/bin/ruv`

## Test at risk
`cache::tests::test_package_cache_path_format` — will fail if `src/cache.rs:51` not updated
