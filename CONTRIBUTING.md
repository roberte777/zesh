# Release Process

1. **Create a Release Branch**:
   - When ready to prepare a release, create a new branch named `release-v0.2` from `main`.

1. **Update Version for Release**:
   - In the `release-v0.2` branch, update the version to `0.2.0`.

1. **Finalize the Release**:
   - Commit the version change.
   - Tag this commit as `v0.2.0`.
   - Push the `release-v0.2` branch and the tag to the remote repository.

1. **Publish the Release**:
   - Publish the new version to crates.io.
   - Create a GitHub release for the `v0.2.0` tag.
   - Include the changelog since the last stable release in the GitHub release description.

1. **Handling Bug Fixes**:
   - For any bug fixes after the release:
      - The bug is fixed on the `main` branch.
      - The `release-v0.2` branch is checked out.
      - All bug fixes are cherry-picked from main to the v0.2 branch.
      - The patch version is bumped.
      - Follow previous steps to finalize the release and publish.
