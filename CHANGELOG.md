# Changelog

All notable changes to Alacritty are documented in this file.
The sections should follow the order `Packaging`, `Added`, `Changed`, `Fixed`
and `Removed`.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## 0.3.0-dev

### Added

- Checks if the directory is a git directory. If so, adds the path to the
directory opened in the session name: base_subfolder_folder
- Pass zellij flags when creating a new session

### Fixed

- Removed extra output from zesh list command. This enables users to be able
to use zesh list in tandem with other cli tools like fzf
