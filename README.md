# apogee

**apogee** — Orbit’s companion that **emits cross‑shell config** (aliases, PATH, env, functions) from smart system detection, so your shell needs just a **single `eval`**.

> status: pre‑alpha (scaffolding)

## quick start (planned)

```sh
# zsh/bash
eval "$(apogee)"
```

```powershell
# PowerShell
&{ $out = apogee; Invoke-Expression $out }
```

The current binary prints a safe placeholder so eval won’t break while you iterate.

## goals

- one binary → consistent output for zsh, bash, and PowerShell (fish later)
- capability detection (apps, paths, Houdini flags, etc.)
- idempotent, conditional emissions
- zero‑config bootstrap: single eval line in `~/.zshrc`, `~/.bashrc`, or `$PROFILE`
- snapshot tests for emitted scripts

## roadmap (sketch)

- [ ] detect: PATH sources, tool presence (houdini, python, etc.)
- [ ] emit: shell‑specific code generation (zsh/bash/pwsh; fish later)
- [ ] profiles: minimal, verbose, debug modes
- [ ] tests: golden snapshots for emitted scripts
- [ ] releases: cross‑platform artifacts

## dev setup

```sh
# from repo root
cargo run --quiet
# or
cargo build
```

The scaffolded `main.rs` prints a harmless placeholder and a sample alias.

## license

MIT
