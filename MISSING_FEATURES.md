# Missing Features Analysis

Comparison of zesh's current implementation against its inspiration
[sesh](https://github.com/joshmedeski/sesh) and general session manager
expectations. Organized by priority.

---

## Critical / Self-Identified Gaps

These are gaps the README itself calls out under **"Missing Essentials"**.

### 1. Pass extra arguments to `git clone`

**Current**: `zesh clone` hardcodes `git clone <url> <path>` with no way to
forward flags like `--depth 1`, `--branch`, `--recurse-submodules`, etc.

**Location**: `zesh/src/main.rs:126-130` — the `Command::new("git")` call only
passes `clone`, the URL, and the clone path.

**Expected**: Accept arbitrary trailing arguments (`--`) and forward them to the
underlying `git clone` invocation.

### 2. `zesh clone` doesn't use the `zesh_git` crate

**Current**: The `Clone` command in `main.rs` shells out to `git clone`
directly (`Command::new("git")`) instead of using the `zesh_git::Git` trait and
`RealGit` implementation that already exists and has a `clone()` method.

**Location**: `zesh/src/main.rs:126-136` vs `zesh_git/src/lib.rs:60-73`

**Impact**: Bypasses the abstraction layer, making the clone path untestable and
inconsistent with the rest of the codebase (connect uses `ConnectService` with
injected traits).

### 3. `list` command deduplication

**Current**: `zesh list` prints all zoxide paths, then all zellij session names.
If a session was created from a zoxide directory, the same project appears
twice — once as a path and once as a session name.

**Location**: `zesh/src/main.rs:80-95`

**Expected**: Deduplicate entries so each project appears once, preferring the
session representation when one exists (sesh deduplicates across sources).

---

## Incomplete / WIP Commands

### 4. `last` — Connect to last-used session

**Current**: Listed as WIP in the README (`🔄 Connect to the last used session
(WIP)`) but has no command definition, no subcommand variant, and no
implementation anywhere in the codebase.

**Expected** (per sesh): A `zesh last` command that switches to the
most-recently-used session. Requires tracking session history (e.g., writing the
previous session name to a state file on each `connect` / `attach`).

### 5. `root` — Show root directory of current session

**Current**: The implementation just prints `pwd`. It does not resolve the
session back to the directory it was created from, nor does it use git to find
the repository root.

**Location**: `zesh/src/main.rs:154-165`

**Expected**: Map the current zellij session name back to its originating
directory (possibly using git worktree / top-level detection), rather than
returning the working directory of the zesh process itself.

### 6. `preview` — Preview a session or directory

**Current**: Minimal skeleton — prints "Session: <name>" for sessions (no actual
detail), and a flat `dir`/`file`/`other` listing for directories. No tree view,
no git status, no session metadata.

**Location**: `zesh/src/main.rs:168-199`, `preview_directory()` at line 215-234

**Expected** (per sesh): Show richer information — git branch, recent commits,
file tree, number of panes/tabs for active sessions — so the preview is useful
in an fzf `--preview` window.

---

## Missing Compared to sesh

### 7. Configuration file support (`sesh.toml` equivalent)

**Current**: No configuration file. All behavior is hardcoded or passed via CLI
flags.

**Expected**: A config file (e.g., `~/.config/zesh/zesh.toml`) that can define:
- Default sessions with startup commands/layouts
- Session blacklist (directories to exclude from list)
- Default zellij options (layout, config path)
- Custom sort ordering for `list`

### 8. `list` filtering flags

**Current**: `zesh list` dumps everything with no filtering. No way to show only
active sessions, only zoxide directories, or only configured sessions.

**Expected** (sesh flags): `-t` (sessions only), `-z` (zoxide only), `-c`
(configured only), `-d` (directories only), `-H` (hide headers/section
labels).

### 9. Shell completions

**Current**: No shell completion generation despite using `clap` which supports
it out of the box via `clap_complete`.

**Expected**: A `zesh completion <shell>` subcommand (or build-time generation)
for bash, zsh, fish, and PowerShell. This is nearly free with clap.

### 10. Icons support

**Current**: No icon support for session/directory listings.

**Expected** (per sesh): Optional icons in `list` output via `--icons` / `-i`
flag, with per-session icon configuration.

### 11. Startup / connect commands

**Current**: When creating a new session, zesh just launches zellij. There's no
hook to run a command after the session starts (e.g., `npm run dev`, open
editor).

**Expected** (per sesh): Per-session or per-directory startup commands defined
in config, executed after session creation.

---

## Implementation Gaps

### 12. `parse_tabs_json` is a no-op

**Current**: The function at `zellij_rs/src/lib.rs:300-313` always returns an
empty `Vec<Tab>`. The comment says to use `serde_json` but it was never
implemented. This means `list_tabs()` is non-functional.

**Impact**: Any future feature that needs tab introspection (e.g., richer
preview, session details) will silently return no data.

### 13. `clone` doesn't use `ConnectService`

**Current**: The `Clone` command in `main.rs` duplicates session-creation logic
(lines 107-152) instead of delegating to `ConnectService::connect_to_directory`
after cloning. This means:
- No git-aware session naming for cloned repos
- No fallback logic
- Inconsistent behavior between `connect` and `clone`

### 14. No session kill/delete command

**Current**: `ZellijOperations` defines `kill_session()` and the mock
implements it, but there is no `zesh kill` or `zesh delete` CLI command.

**Expected**: A way to kill sessions without dropping to raw `zellij
kill-session`.

### 15. No `zoxide remove` integration

**Current**: `ZoxideOperations` only supports `add`, `list`, and `query`. No
`remove` operation. If a directory is deleted or a user wants to clean up stale
entries, there's no way to do it through zesh.

---

## Quality / Polish

### 16. `list` output has no session indicators

**Current**: Sessions are printed as bare names with no indication of which is
the current/active session, despite the `Session` struct tracking `is_current`.

**Location**: `zesh/src/main.rs:92-94`

### 17. `list` output ordering is undefined

**Current**: Zoxide entries come out sorted by score (highest first), but
sessions come out in HashMap iteration order (effectively random). No unified
sorting strategy.

### 18. Error handling inconsistency in `clone`

**Current**: `clone` prints errors and returns `Ok(())` on failure
(`main.rs:132-136`) instead of propagating errors. This silently succeeds from
the caller's perspective.

### 19. No `--help` examples in subcommands

**Current**: Subcommand help text is minimal (e.g., `/// List sessions`). No
usage examples in `#[clap(long_about)]` or `#[clap(after_help)]`.

---

## Summary Table

| # | Feature | Status | Priority |
|---|---------|--------|----------|
| 1 | Pass args to `git clone` | Missing | Critical |
| 2 | Clone uses raw git, not `zesh_git` | Inconsistency | Critical |
| 3 | List deduplication | Missing | Critical |
| 4 | `last` command | Not implemented | High |
| 5 | `root` command | Stub only | High |
| 6 | `preview` command | Skeleton only | High |
| 7 | Config file support | Missing | High |
| 8 | `list` filtering flags | Missing | Medium |
| 9 | Shell completions | Missing | Medium |
| 10 | Icons support | Missing | Low |
| 11 | Startup commands | Missing | Medium |
| 12 | `parse_tabs_json` no-op | Broken | Medium |
| 13 | Clone doesn't use ConnectService | Inconsistency | Medium |
| 14 | Session kill command | Missing | Medium |
| 15 | Zoxide remove integration | Missing | Low |
| 16 | List: no current-session indicator | Missing | Low |
| 17 | List: undefined ordering | Missing | Low |
| 18 | Clone error handling | Bug | Medium |
| 19 | Subcommand help examples | Missing | Low |
