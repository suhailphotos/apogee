# apogee

**apogee** is a cross-shell *init emitter*.

You keep **one** TOML config, and `apogee` prints the shell code needed to set:

- env vars
- PATH edits
- aliases
- functions
- optional templates / hooks

It supports **zsh, bash, fish, and PowerShell**, and can enable modules automatically based on runtime detection (paths, commands, files, env vars, versions).

---

## Why apogee

Most dotfile setups drift because they’re shell-specific.

apogee flips that: you describe *intent* in `config.toml` and apogee emits the right syntax for the active shell.

---

## Install

### From crates.io (recommended)

```sh
cargo install apogee
```

Make sure Cargo bin is on your PATH:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify:

```sh
apogee --version
```

### From source (dev)

```sh
cargo install --path .
```

---

## Quick start

### 1) Initialize config + shell hook

Run:

```sh
apogee init
```

This will:

- create `~/.config/apogee/config.toml` (only if missing)
- create `~/.config/apogee/{functions,hooks,templates}/` starter dirs
- append a guarded “load apogee” block to your shell rc/profile file (only if missing)

Then restart your shell (or source your rc file).

### 2) Manual load (if you don’t want `init` to touch rc files)

Use one of these instead:

**zsh / bash**
```sh
eval "$(APOGEE_SHELL=zsh apogee)"
# or
eval "$(APOGEE_SHELL=bash apogee)"
```

**fish**
```fish
env APOGEE_SHELL=fish apogee | source
```

**PowerShell**
```powershell
$env:APOGEE_SHELL = "pwsh"
(& apogee) | Out-String | Invoke-Expression
```

---

## Configuration

Default location:

- `~/.config/apogee/config.toml`

apogee is **modules-first**:

- each module has *detect rules* (paths/commands/files/env/version)
- modules emit output only when active
- modules can depend on other modules (requires)

Out of the box, the starter config includes a minimal baseline plus a Dropbox module (`DROPBOX` only) and common CLI tooling patterns.

---

## Typical usage

Once your shell loads apogee (via `apogee init` or manual load), you usually **don’t** run `apogee` again in that session.

apogee emits functions/aliases once at shell startup.

---

## Testing in a clean environment

These launch shells with a minimal environment so you can validate emissions without your normal dotfiles interfering.

> Tip: keep `TERM` so `clear` works.

### zsh (clean)

```sh
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" \
  TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" \
  XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" \
  APOGEE_SHELL=zsh \
  zsh -f
```

Inside:

```sh
eval "$(apogee)"
```

### bash (clean)

```sh
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" \
  TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" \
  XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" \
  APOGEE_SHELL=bash \
  bash --noprofile --norc
```

Inside:

```sh
eval "$(apogee)"
```

### fish (clean)

```sh
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" \
  TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" \
  XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" \
  APOGEE_SHELL=fish \
  fish --no-config
```

Inside:

```fish
apogee | source
```

### PowerShell (clean)

```sh
PWSH_BIN="$(command -v pwsh || command -v powershell)"

env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" \
  TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" \
  XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" \
  APOGEE_SHELL=pwsh \
  "$PWSH_BIN" -NoProfile
```

Inside:

```powershell
(& apogee) | Out-String | Invoke-Expression
```

---

## Troubleshooting

### “apogee init” didn’t modify my shell file

`init` only edits known rc/profile files:

- zsh: `~/.zshrc`
- bash: `~/.bashrc`
- fish: `${XDG_CONFIG_HOME:-~/.config}/fish/config.fish`
- pwsh: `${XDG_CONFIG_HOME:-~/.config}/powershell/Microsoft.PowerShell_profile.ps1`

If apogee can’t detect your shell, it will print what to add manually.

### Avoiding command shadowing

If you create an alias/function named `apogee`, you can accidentally shadow the binary.

When you need the actual executable:

- zsh/bash: `command apogee`
- fish: `command apogee`
- PowerShell: `& (Get-Command apogee -CommandType Application).Source`

---

## Developer workflow

```sh
cargo fmt
cargo clippy
cargo test
```

Print output for a specific shell:

```sh
APOGEE_SHELL=zsh  apogee | sed -n '1,200p'
APOGEE_SHELL=bash apogee | sed -n '1,200p'
APOGEE_SHELL=fish apogee | sed -n '1,200p'
APOGEE_SHELL=pwsh apogee | sed -n '1,200p'
```

Check packaging before publishing (ensures `assets/**` is included):

```sh
cargo package --list | rg 'assets/default_config.toml'
cargo publish --dry-run
```

---

## Roadmap (high level)

- More polished starter templates
- Better PowerShell profile discovery on Windows
- More robust version detection + conditional emits
- Optional `apogee doctor` for config + environment diagnostics

---

## License

MIT

