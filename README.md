# arrrv

> A fast R package manager, written in Rust. The R equivalent of [`uv`](https://github.com/astral-sh/uv).

---

## Motivation

R package management is slow, fragmented, and requires too many tools:

- `install.packages()` — sequential, no caching, no lockfiles
- `renv` — lockfiles but slow restoration, no binary cache
- `pak` — faster installs but no lockfile or version management
- `rig` — R version management, but a separate tool

`arrrv` replaces all of them with a single fast binary.

## Goals

- **Fast** — parallel downloads, global binary cache, hard-links into project libraries (zero-copy installs)
- **Reproducible** — lockfile-based installs, exact version pinning
- **Integrated** — package management + R version management in one tool
- **Simple** — one binary, familiar `uv`-style workflow

## Usage

### Project setup

Create an `arrrv.toml` in your project directory:

```toml
[project]
name = "my-analysis"
version = "0.1.0"
r-version = ">=4.3"
dependencies = [
    "ggplot2",
    "dplyr",
]
```

### Commands

```sh
# Resolve dependencies and write arrrv.lock
arrrv lock

# Install exact versions from arrrv.lock
arrrv sync

# Install a package and its dependencies (without modifying arrrv.toml)
arrrv install ggplot2

# Run a script using the project library
arrrv run Rscript analysis.R
arrrv run -- -e "library(ggplot2)"
```

### Typical workflow

```sh
# First time — resolve and install
arrrv lock
arrrv sync

# After editing arrrv.toml — re-resolve and reinstall
arrrv lock
arrrv sync

# Colleague clones the repo — restore exactly from lockfile
arrrv sync
```

### Flags

```sh
arrrv --verbose sync   # show per-package source (cache vs download)
```

## How it works

- **`arrrv lock`** fetches the CRAN package index, resolves all transitive dependencies using the PubGrub algorithm, and writes `arrrv.lock` with exact versions, the full dependency graph, and a per-package [RSPM](https://packagemanager.posit.co) snapshot URL derived from each package's CRAN upload date. This makes installs reproducible even as CRAN evolves.
- **`arrrv sync`** reads `arrrv.lock` directly — no CRAN fetch required — and installs packages into `.arrrv/library/` from the pinned RSPM binary URLs. Packages are downloaded once to a global cache (`~/Library/Caches/arrrv/` on macOS) and hard-linked into the project library, so repeated installs across projects are instant.

## Comparison

| | `install.packages` | `renv` | `pak` | **arrrv** |
|---|---|---|---|---|
| Parallel downloads | ❌ | ❌ | ✅ | ✅ |
| Global binary cache | ❌ | ❌ | ❌ | ✅ |
| Lockfile | ❌ | ✅ | ❌ | ✅ |
| Reproducible binary installs | ❌ | ⚠️ source only | ❌ | ✅ |
| Lock/sync separation | ❌ | ❌ | ❌ | ✅ |
| R version management | ❌ | ❌ | ❌ | 🚧 planned |

## Status

Working MVP on macOS (arm64 + x86_64). Active development — see the [GitHub issues](https://github.com/A-Fisk/arrrv/issues) for the roadmap.

**What works:**
- `arrrv lock` — PubGrub dependency resolution + write lockfile with pinned RSPM binary URLs
- `arrrv sync` — restore from lockfile using pinned RSPM binaries (no CRAN fetch on warm runs)
- `arrrv install` — one-off package install
- `arrrv run` — run scripts with the project library
- Version constraint solving (e.g. `"ggplot2 (>= 3.4)"` in `arrrv.toml`)
- Global package cache with hard-linking
- 50 unit tests, CI on GitHub Actions

**Coming next:**
- `arrrv add` / `arrrv remove`
- Bioconductor package support
- R version management

## Development

Requires Rust (install via [rustup](https://rustup.rs)):

```sh
git clone https://github.com/A-Fisk/arrrv
cd arrrv
cargo build
cargo test
```

## License

MIT
