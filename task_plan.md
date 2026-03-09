# Task Plan: Rename arrrv → ruv

## Goal
Rename the CLI tool from `arrrv` to `ruv` across the entire codebase, including source code, config, scripts, formulas, and documentation.

## Decisions
- Binary: `arrrv` → `ruv`
- Config file: `arrrv.toml` → `ruv.toml`
- Lockfile: `arrrv.lock` → `ruv.lock`
- Project library dir: `.arrrv/library` → `.ruv/library`
- Global cache dir: `~/...caches/arrrv` → `~/...caches/ruv`
- Homebrew formula: `formula/arrrv.rb` → `formula/ruv.rb`
- GitHub repo rename is OUT OF SCOPE (follow-up task)

## Phases

### Phase 1: Rust Source (validate with cargo test)
- [x] Step 1: `Cargo.toml` — `name = "arrrv"` → `name = "ruv"`
- [x] Step 2: `src/cache.rs` — path join, doc comment, test assertion
- [x] Step 3: `src/config.rs` — `CONFIG_FILE` constant, 2 error strings, comments
- [x] Step 4: `src/lockfile.rs` — 3 hardcoded filenames, error/print strings, doc comments
- [x] Step 5: `src/main.rs` — `LIB_DIR`, clap `name`, ~12 user-facing strings, doc comments
- [x] Step 6: `src/resolver.rs` — doc/test comments only
- [x] **Checkpoint**: `cargo test` — 59/59 passed ✅

### Phase 2: Infrastructure & Distribution
- [x] Step 7: `.github/workflows/release.yml` — tarball dir/filename, artifact name
- [x] Step 8: `scripts/install.sh` — binary name, tarball filename, echo strings
- [x] Step 9: `formula/arrrv.rb` → `formula/ruv.rb` — `git mv` + class name + all URLs

### Phase 3: Config Artifacts
- [x] Step 10: `.gitignore` — 3 path entries
- [x] Step 11: `arrrv.toml` → `ruv.toml` — `git mv` (content unchanged)
- [x] Step 12: `arrrv.lock` → `ruv.lock` — `git mv`

### Phase 4: Documentation
- [x] Step 13: `README.md` — all occurrences
- [x] Step 14: `DISTRIBUTION.md` — all occurrences
- [x] Step 15: `mvp_plan.md` — all occurrences
- [x] Step 16: `project_plan.md` — all occurrences
- [x] Step 17: `test_coverage.md` — all occurrences

## Status
✅ Complete — all 17 steps done, 59/59 tests pass, no `arrrv` references remain
