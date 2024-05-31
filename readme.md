
<h1 align="center">
  <a href="https://github.com/ExtremelyRyan/atomic">
    <!-- Please provide path to your logo here -->
    <img src=".media/atomic 2.png" alt="Logo" width=500px, height=500px>
  </a>
</h1>

<div align="center">
  <h1>Atomic</h1>
  <br />
  <a href="#about"><strong>Explore the screenshots »</strong></a>
  <br />
  <br />
  <a href="https://github.com/ExtremelyRyan/atomic/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+">Report a Bug</a>
  <a href="https://github.com/ExtremelyRyan/atomic/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+">Request a Feature</a>
  <a href="https://github.com/ExtremelyRyan/atomic/issues/new?assignees=&labels=question&template=04_SUPPORT_QUESTION.md&title=support%3A+">Ask a Question</a>
</div>

<div align="center">
<br />

[![Project license](https://img.shields.io/github/license/ExtremelyRyan/atomic.svg?style=flat-square)](LICENSE)
[![code with love by ExtremelyRyan](https://img.shields.io/badge/%3C%2F%3E%20with%20%E2%99%A5%20by-ExtremelyRyan-ff1414.svg?style=flat-square)](https://github.com/ExtremelyRyan)

<H3 align="center">this project is still in rapid development, and is prone to breaking changes on main.</H3>
</div>

## What's Atomic?

Atomic is a command-line tool designed to streamline the process of making "atomic" commits. It addresses the challenge of remembering to save frequent snapshots of your code without disrupting your workflow. By defining custom commands in an atomic.toml file located in your project's root directory, Atomic allows you to execute your desired actions while automatically creating local commit snapshots in Git. This ensures that your changes are captured efficiently and without interrupting your focus.

### Built With

Rust, because I like it.Also with
[clap](https://lib.rs/crates/clap),
[git2](https://lib.rs/cratesgit2),
[thiserror](https://lib.rs/crates/thiserror),
[toml](https://lib.rs/crates/toml)

## Getting Started

### Prerequisites

Rust MSRV: 1.74
Windows 10/11
not tested on linux (yet)

### Installation

> **[?]**
> TODO

## Usage

### Default Commands
**[!]** all commands are modifiable from the project root `atomic.toml` file.

- For setting up a new project simply run `atomic init` in your project root directory, which will create a 
`atomic.toml` file with some defaults (for rust commands), as well as a few examples.

the following commands are considered the "default" that will apply to most projects. 

- `atomic run` 
- `atomic test` 
- `atomic build` 

this is how they appear in the toml file
```toml
# default commands
[default]
build = "echo build"
test  = "echo test"
run   = "echo run"
```

### Custom Commands
custom commands are for everything else you need to do that you **also want a local git commit to happen.**

examples from the template file
```toml
# custom commands go here
[custom]
check      = "cargo check"
check      = "echo check2"
clippy     = "cargo clippy"
clippy_max = "cargo clippy --all-targets --all-features --workspace -- -D warnings"
doc        = "cargo doc --no-deps --document-private-items --all-features --workspace"
test-all   = "cargo test --all-features --workspace"

# chain several custom commands together, regardless if they are declared.
chain = ["check", "clippy", "cargo fmt"]
```
Note: if two keys are identical, atomic will default to execute the first command found.




## Roadmap

See the [open issues](https://github.com/ExtremelyRyan/atomic/issues) for a list of proposed features (and known issues).

- [Top Feature Requests](https://github.com/ExtremelyRyan/atomic/issues?q=label%3Aenhancement+is%3Aopen+sort%3Areactions-%2B1-desc) (Add your votes using the 👍 reaction)
- [Top Bugs](https://github.com/ExtremelyRyan/atomic/issues?q=is%3Aissue+is%3Aopen+label%3Abug+sort%3Areactions-%2B1-desc) (Add your votes using the 👍 reaction)
- [Newest Bugs](https://github.com/ExtremelyRyan/atomic/issues?q=is%3Aopen+is%3Aissue+label%3Abug)

## Support

Reach out to the maintainer at one of the following places:

- [GitHub issues](https://github.com/ExtremelyRyan/atomic/issues/new?assignees=&labels=question&template=04_SUPPORT_QUESTION.md&title=support%3A+)
- Contact options listed on [this GitHub profile](https://github.com/GITHUB_USERNAME)

## Project assistance

If you want to say **thank you** or/and support active development of Atomic:

- Add a [GitHub Star](https://github.com/ExtremelyRyan/atomic) to the project.
- Tweet about the Atomic.
- Write interesting articles about the project on [Dev.to](https://dev.to/), [Medium](https://medium.com/) or your personal blog.

## Authors & contributors

The original setup of this repository is by [Ryan](https://github.com/ExtremelyRyan).
