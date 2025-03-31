
<h1 align="center">
  <a href="https://github.com/ExtremelyRyan/atomic">
    <!-- Please provide path to your logo here -->
    <img src=".media/atomic 2.png" alt="Logo" width=500px, height=500px>
  </a>
</h1>

<div align="center">
  <h1>Atomic</h1>  
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

### **Built With**

**Rust**, because I like it. 

## Getting Started

### Prerequisites

- Rust MSRV: 1.74

- Windows 10/11

- not tested on linux (yet)

- any scripting language(s) you intend on using for pre/post hooks

### Installation


1. **Install from Source**

```bash
git clone https://github.com/ExtremelyRyan/atomic.git
cd atomic
cargo install --path .
```

2. **From Crates.io**

```bash
cargo install cargo-atomic
```


## Usage 

- For setting up a new project simply run `atomic init` in your project root directory, which will create a 
`atomic.toml` file with some defaults (for rust commands), as well as a few examples.



### Custom Commands
custom commands are for everything else you need to do that you **also want a local git commit to happen.**

examples from the template file
```toml 
[custom.check]
command = "cargo check"

[custom.clippy]
before  = "echo Running Clippy"
command = "cargo clippy"
after   = "echo Clippy finished"

[custom.tw]
command = "cargo test --all-features --workspace"

[custom.clippy_max]
command = "cargo clippy --all-targets --all-features --workspace -- -D warnings"

[custom.doc]
command = "cargo doc --no-deps --document-private-items --all-features --workspace --open"

# Chain multiple commands, declared or raw
[custom.chain]
command = ["check", "tw", "clippy_max"]
```
**Note**: no two command keywords can be the same

Here's the short on setting up a new custom command

```toml
# Your new atomic command
# replace "test" with what you want to call this action
# i.e. Atomic test
[custom.test]
before = "path/to/script" # prehook action
command = "cargo test" # main command
after = "path/to/script" or command line action # i.e. some clean up commands 

```


### Run a plugin:

```bash
atomic --plugin docs
```

If the plugin has `silent = true`, output is logged to `atomic-logs/docs.log`.

here's an example"
```toml
[plugin.generate-docs]
script = "./scripts/test.py" 
args = ["hello", "from", "Atomic!"] 
silent = true   # optional, defaults to false
```


## 💾 Auto-committing

After running a command or plugin, Atomic will automatically commit any changes in the repository with a timestamp-based commit message. The commit message will look like this:

```
[2024-03-27 13:52:12] command: cargo fmt
```

You don’t need to do anything else — it just works. this helps
you stay focused on what you're building and not worry about saving your progress. 

Supports:
- `.sh`, `.bat`, `.cmd`, `.ps1`, `.py`, `.exe`
- Auto-switches based on OS
- Log output to `atomic-logs/<plugin>.log` when `silent = true` on any custom command. 

---

## 🔁 Pre/Post Hooks

Any custom command can have a `before` or `after` script:

```toml
[custom.test]
before  = "echo Starting tests"
command = "cargo test"
after   = "echo All done"
```

---

## 🧠 Templates

Quickly generate starter configs:

```bash
atomic init --template rust
atomic init --template example
```

Available templates:
- `rust`
- `example`

---

## 💬 Flags

| Flag            | Description                             |
|-----------------|-----------------------------------------|
| `--init`        | Create a new `atomic.toml` file         |
| `--template`    | Select a template to initialize with    |
| `--list`        | List all available commands             |
| `--plugin`      | Run a plugin defined in `atomic.toml`   |

---


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

Created by [ExtremelyRyan](https://github.com/ExtremelyRyan).
