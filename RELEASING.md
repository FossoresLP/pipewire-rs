# Release process

- push a `release` branch, based on up to date `main`, to your personal repo and check that the CI pipeline passes. It should run extra jobs such as `outdated` which will check if our dependencies are up to date.

- use [cargo-workspaces](https://crates.io/crates/cargo-workspaces) to create and publish the release:
`cargo workspaces publish`.