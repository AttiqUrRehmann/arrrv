# arrrv

> A fast R package manager, written in Rust. The R equivalent of [`uv`](https://github.com/astral-sh/uv).

---

## Motivation

R package management is slow, fragmented, and requires too many tools:

- `install.packages()` — sequential, no caching, no lockfiles
- `renv` — lockfiles but slow restoration
- `pak` — faster installs but no lockfile or version management
- `rig` — R version management, but a separate tool

`arrrv` replaces all of them with a single fast binary.

## Goals

- **Fast** — parallel downloads, global binary cache, hard-links into project libraries
- **Reproducible** — lockfile-based installs
- **Integrated** — package management + R version management in one tool
- **Simple** — one binary, no R installation required to use `arrrv` itself

## Planned CLI

```sh
arrrv install ggplot2          # install a package
arrrv add ggplot2              # add to project + update lockfile
arrrv sync                     # restore from lockfile
arrrv run Rscript analysis.R   # run with project library

arrrv r install 4.4.1          # install an R version
arrrv r list                   # list installed R versions
arrrv r pin 4.4.1              # pin R version for this project
```

## Status

Early development. See [`mvp_plan.md`](mvp_plan.md) for the current build plan and
[`project_plan.md`](project_plan.md) for the full vision.

**MVP goal:** `arrrv install ggplot2` fetches, resolves, and installs a package and
all its dependencies into a local library directory on macOS.

## Development

Requires Rust (install via [rustup](https://rustup.rs)):

```sh
git clone https://github.com/yourname/arrrv
cd arrrv
cargo run
```

## License

MIT
