# apogee

**apogee** emits cross-shell shell initialization (env vars, PATH edits, aliases, functions, and templates) from a single TOML config and runtime detection.

- Config: `~/.config/apogee/config.toml`
- Typical usage: load once per shell session, then use the emitted functions/aliases.

> Status: pre-alpha. Expect breaking changes.

---

## Install

### From crates.io (recommended)

```sh
cargo install apogee
```

Ensure `~/.cargo/bin` is on your PATH:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify:

```sh
type -a apogee
```

### From source (dev)

From the repo root:

```sh
cargo install --path .
```

### Run without installing (dev)

```sh
cargo run --quiet
```

---

## Quick start

### zsh / bash

```sh
eval "$(apogee)"
```

### fish

```fish
apogee | source
```

### PowerShell

```powershell
. ([ScriptBlock]::Create((& apogee | Out-String)))
```

---

## Testing in a clean environment (recommended)

These commands launch each shell with a minimal environment so you can validate Apogee emissions without your normal dotfiles interfering.

> Note: `TERM` is intentionally provided here so `clear` works in the clean shell.
> In your final setup, terminal defaults should come from your terminal/rc files, not from apogee.

### zsh (clean)

```sh
env -i   HOME="$HOME" USER="$USER" LOGNAME="$USER"   TERM="${TERM:-xterm-256color}" COLORTERM="${COLORTERM:-truecolor}"   LANG="${LANG:-en_US.UTF-8}"   PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"   XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" XDG_DATA_HOME="$HOME/.local/share"   APOGEE_SHELL=zsh   zsh -f
```

Inside the shell:

```sh
eval "$(apogee)"
```

### bash (clean)

```sh
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER"   TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}"   PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"   XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache"   APOGEE_SHELL=bash   bash --noprofile --norc
```

Inside the shell:

```sh
eval "$(apogee)"
```

### fish (clean)

```sh
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER"   TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}"   PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"   XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache"   APOGEE_SHELL=fish   fish --no-config
```

Inside fish:

```fish
apogee | source
```

### PowerShell (clean)

```sh
PWSH_BIN="$(command -v pwsh || command -v powershell)"

env -i HOME="$HOME" USER="$USER" LOGNAME="$USER"   TERM="${TERM:-xterm-256color}" LANG="${LANG:-en_US.UTF-8}"   PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"   XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache"   APOGEE_SHELL=pwsh   "$PWSH_BIN" -NoProfile
```

Inside PowerShell:

```powershell
. ([ScriptBlock]::Create((& apogee | Out-String)))
```

---

## Quick smoke checks (after loading)

These help confirm apogee actually loaded.

### zsh / bash

```sh
type pkg
type python_projects
echo "$PACKAGES"
```

### fish

```fish
functions -q pkg; and echo "pkg ok"
functions -q python_projects; and python_projects
echo $PACKAGES
```

### PowerShell

```powershell
Get-Command pkg -ErrorAction SilentlyContinue
Get-Command python_projects -ErrorAction SilentlyContinue
$env:PACKAGES
```

---

## Avoiding command shadowing during testing

Because apogee may emit an alias/function named `apogee` (for `cd`), you may accidentally shadow the binary.

When you want to be certain you’re running the installed binary:

### zsh / bash

```sh
command apogee
```

### fish

```fish
command apogee | head -n 5
```

### PowerShell

```powershell
& (Get-Command apogee -CommandType Application).Source | Out-String
```

If in doubt, call it explicitly:

- mac/linux: `$HOME/.cargo/bin/apogee`

---

## Developer workflow

### Format + check

```sh
cargo fmt
cargo clippy
cargo test
```

### Print output for a specific shell

```sh
APOGEE_SHELL=zsh  apogee | sed -n '1,200p'
APOGEE_SHELL=bash apogee | sed -n '1,200p'
APOGEE_SHELL=fish apogee | sed -n '1,200p'
APOGEE_SHELL=pwsh apogee | sed -n '1,200p'
```

---

## License

MIT

