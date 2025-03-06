# Contributing to Zesh

Thank you for your interest in contributing to Zesh! This document provides
guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct.
Please be respectful and considerate of others when participating in
discussions, submitting issues, or contributing code.

## How to Contribute

There are many ways to contribute to Zesh:

- Reporting bugs
- Suggesting enhancements
- Writing documentation
- Submitting code changes
- Helping other users with their issues
- Reviewing pull requests

### Reporting Bugs

If you find a bug, please create an issue with the following information:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected behavior vs. actual behavior
- Any relevant logs or error messages
- Your environment (OS, Rust version, Zellij version, etc.)

### Suggesting Enhancements

Feature requests are welcome! Please create an issue with:

- A clear, descriptive title
- Detailed explanation of the feature
- Use cases and benefits
- Any implementation ideas you have

### Development Workflow

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/zesh.git`
3. Create a new branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Format your code: `cargo fmt`
7. Lint your code: `cargo clippy`
8. Commit your changes: `git commit -m "Add your meaningful commit message"`
9. Push to your fork: `git push origin feature/your-feature-name`
10. Create a pull request

### Pull Request Process

1. Update the README.md or documentation with details of changes to the
interface, if applicable
2. Update the CHANGELOG.md to document your changes
3. Make sure all tests pass and CI checks are green
4. Request a review from a maintainer
5. Respond to any feedback on your pull request

## Style Guidelines

- Follow the Rust style guide and use `cargo fmt` before committing
- Write clear, descriptive commit messages
- Comment complex code sections
- Update documentation when changing functionality

## Release Process

1. **Create a Release Branch**:
   - When ready to prepare a release, create a new branch named `release-vX.Y`
   from `main` (e.g., `release-v0.3`).

2. **Update Version for Release**:
   - In the `release-vX.Y` branch, update the version number in all relevant files:
     - All `Cargo.toml` files in the workspace
     - Any other version references in the code
   - Update CHANGELOG.md with the new version and release notes

3. **Finalize the Release**:
   - Commit the version change
   - Tag this commit as `vX.Y.Z` (e.g., `v0.3.0`)
   - Push the `release-vX.Y` branch and the tag to the remote repository

4. **Publish the Release**:
   - Publish the new version to crates.io: `cargo publish`
   - Create a GitHub release for the `vX.Y.Z` tag
   - Include the changelog since the last stable release in the GitHub release
   description

5. **Handling Bug Fixes**:
   - For any bug fixes after the release:
     - The bug is fixed on the `main` branch first
     - The `release-vX.Y` branch is checked out
     - Bug fixes are cherry-picked from main to the release branch
     - The patch version is bumped (e.g., from `0.3.0` to `0.3.1`)
     - Follow previous steps to finalize the release and publish

## License

By contributing to Zesh, you agree that your contributions will be licensed
under the same MIT license that covers the project.

## Questions?

If you have any questions about contributing, please open an issue or reach out
to the maintainers.

Thank you for your contributions!
