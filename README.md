# Zesh

A zellij session manager with zoxide integration, inspired by tmux-sesh.

## Features

- List active zellij sessions
- Connect to the last used session
- Create and connect to sessions based on directories
- Clone git repositories and set up sessions
- Fuzzy directory searching with zoxide integration
- Preview sessions and directories

## Installation

```bash
cargo install --locked zesh
```

## Requirements

- [zellij](https://zellij.dev/) - Terminal multiplexer
- [zoxide](https://github.com/ajeetdsouza/zoxide) - Smarter cd command
- [git](https://git-scm.com/) - Version control (optional, for clone command)

## Usage

```
# List active sessions
zesh list
zesh l

# List all recent directories from zoxide
zesh list --all

# Connect to the last session
zesh last
zesh L

# Connect to a specific session or directory
zesh connect <name>
zesh cn <name>

# Clone a git repo and create a session
zesh clone https://github.com/username/repo
zesh cl https://github.com/username/repo

# Show the root directory of the current session
zesh root
zesh r

# Preview a session or directory
zesh preview <name>
zesh p <name>

# Display help
zesh help
zesh h
```

## Why Zesh?

Zesh combines the power of zellij (terminal multiplexer) and zoxide (directory jumper) to provide a seamless session management experience. It's designed for developers who frequently work on multiple projects and want to quickly jump between them.

The name "zesh" is a combination of:

- **z** from **z**ellij and **z**oxide
- **sh** from se**sh**

## License

MIT
