# Zesh

A zellij session manager with zoxide integration, inspired by [tmux-sesh](https://github.com/joshmedeski/sesh)
by Josh Medeski.

## Features

- List active zellij sessions
- Create and connect to sessions based on zoxide
- Zoxide-powered session detection and management
- Clone git repositories and set up sessions

## Installation

Currently, ths project can be installed with cargo, or the binaries can be found
directly in the [GitHub releases](https://github.com/roberte777/zesh/releases).

```bash
cargo install --locked zesh
```

## Requirements

- [zellij](https://zellij.dev/) - Terminal multiplexer
- [zoxide](https://github.com/ajeetdsouza/zoxide) - Smarter cd command
- [git](https://git-scm.com/) - Version control (optional, for clone command)

## 🚀 Usage

```bash
# Connect to a specific session or directory
zesh connect <name>
zesh cn <name>

# List active sessions (intended to be used with other cli tools, like fzf)
zesh list
zesh l

# Pair the two commands with fzf
zesh cn $(zesh l | fzf)

# Clone a git repo and create a session
zesh clone https://github.com/username/repo
zesh cl https://github.com/username/repo

# Display help
zesh help
zesh --help
zesh -h
```

## Subject to Change

This project is still heavily under development. Currently, some current
features may change, and some essential features have not been added.

## Why Zesh?

Zesh combines the power of zellij (terminal multiplexer) and zoxide
(directory jumper) to provide a seamless session management experience. It's
designed for developers who frequently work on multiple projects and want to
quickly jump between them.

## Credits

This project was inspired by [sesh](https://github.com/joshmedeski/sesh) by
Josh Medeski, a tmux session manager. Huge thanks to Josh for the original
concept that made terminal session management so much more enjoyable!

## AI Usage

I have used AI to write documentation because I am lazy (like this README).
I have also used AI to do some basic feature work while I was out of town on
vacation. The code is probably bad and primarily written by me trying to learn
from how Josh writes code.

## License

MIT
