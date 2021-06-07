# Contributing to `pipewire-rs`

The `pipewire-rs` project welcomes any contribution in the form of code changes, documentation,
bug reports or general feedback.

This document aims to provide some guidance if you are thinking of making a contribution.

If you need help, feel free to open an issue or reach out via IRC at [#pipewire-rs](irc://irc.oftc.net:6667/pipewire-rs) on [OFTC](https://www.oftc.net/).

# Submitting Code

Before you submit any code changes, please ensure that your code matches the following criteria:

- The code is properly formatted using `rustfmt`
- The code produces no compiler or clippy errors/warnings
- No tests fail
- The code adheres to the [rust api guidelines](https://rust-lang.github.io/api-guidelines/) (within reason)
- New files need to have the following license and copyright header at the top:
  ```rust
  // Copyright The pipewire-rs Contributors.
  // SPDX-License-Identifier: MIT
  ```

Copyrights in the `pipewire-rs` project are retained by their contributors.

Feel free to open an incomplete merge request prefixed with `WIP:` or `Draft:` if you want any help or early feedback.

## Creating a merge request
`pipewire-rs` uses a fast-forward merge only policy, which means that your merge requst may need to be rebased onto the head of the upstream branch. \
When creating the merge request, please enable `allow commits from members who can merge to the target branch` checkbox so that project members can take care of this for you.

# Code of Conduct
All contributers are expected to adhere to the [Contributor Covenant Code of Conduct](code_of_conduct.md).
Any violations may be reported by opening a confidential issue
on [GitLab](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/issues/new).
