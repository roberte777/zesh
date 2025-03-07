# 🔥 Zesh

A zellij session manager with zoxide integration, inspired by [tmux-sesh](https://github.com/joshmedeski/sesh)
by Josh Medeski.

## ✨ Features

- 📋 List active zellij sessions
- 📁 Create and connect to sessions based on zoxide
- 🧠 Smart session detection and management
- 🔍 Clone git repositories and set up sessions
- 🔄 Connect to the last used session (WIP)

## 🛠️ Installation

Currently, ths project can be installed with cargo, or the binaries can be found
directly in the [GitHub releases](https://github.com/roberte777/zesh/releases).

```bash
cargo install --locked zesh
```

## 📋 Requirements

- [zellij](https://zellij.dev/) - 🧩 Terminal multiplexer
- [zoxide](https://github.com/ajeetdsouza/zoxide) - 🔍 Smarter cd command
- [git](https://git-scm.com/) - 📦 Version control (optional, for clone command)

## 🚀 Usage

```bash
# 🔗 Connect to a specific session or directory
zesh connect <name>
zesh cn <name>

# 📋 List active sessions (intended to be used with other cli tools, like fzf)
zesh list
zesh l

# Pair the two commands with fzf
zesh cn $(zesh l | fzf)

# 📦 Clone a git repo and create a session
zesh clone https://github.com/username/repo
zesh cl https://github.com/username/repo

# 📂 Show the root directory of the current session
zesh root
zesh r

# 👁️ Preview a session or directory
zesh preview <name>
zesh p <name>

# ❓ Display help
zesh help
zesh h
```

## Subject to Change

This project is still heavily under development. Currently, some current
features may change, and some essential features have not been added.

**Potential to Change**:

- I am currently unsure of how zesh list should operate. The current method is
nice for fzf, but not much else. This may change in the future.

**Missing Essentials**:

- I am missing functionality to pass arguments to git clone and zellij. This
is espeically important, as layouts are heavily used in zellij.

## ❓ Why Zesh?

Zesh combines the power of zellij (terminal multiplexer) and zoxide
(directory jumper) to provide a seamless session management experience. It's
designed for developers who frequently work on multiple projects and want to
quickly jump between them.

## 🙏 Credits

This project was inspired by [sesh](https://github.com/joshmedeski/sesh) by
Josh Medeski, a tmux session manager. Huge thanks to Josh for the original
concept that made terminal session management so much more enjoyable!

## 📜 License

MIT
