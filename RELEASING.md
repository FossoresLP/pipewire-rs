# Release process

- push a `release` branch, based on up to date `main`, to your personal repo and check that the CI pipeline passes. It should run extra jobs such as `outdated` which will check if our dependencies are up to date.

- checkout `main` and use [cargo-workspaces](https://crates.io/crates/cargo-workspaces) to create and publish the release:
`cargo workspaces publish`.

- make sure the commits and tags have been published to the [upstream repo](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/tags). It should be done automatically if `origin` points to this repo.