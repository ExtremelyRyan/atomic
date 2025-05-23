
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

# Atomic

**Atomic** is a fast, minimal CLI tool that automates local Git commits around the scripts you already run — tests, formatters, docs, builds, anything. Define commands once in a TOML file. Let Atomic run them and snapshot your work with zero friction.
  

Use it as:
- A commit automation tool
- A task runner that remembers to save your work
- A Git-integrated wrapper for dev scripts
- A shell-based CI runner for solo workflows

## 🔧 Why Use Atomic?

Atomic solves a common annoyance: you run `cargo test`, `npm run format`, or `./scripts/setup.sh` — but forget to commit. Or you commit inconsistently.

Now, your routine commands can **automatically commit your changes**, so you never lose progress.

## 🚀 Installation

### From Source

```bash
git clone https://github.com/ExtremelyRyan/atomic.git
cd atomic
cargo install --path .
```

### From Crates.io

```bash
cargo install cargo-atomic
```

> Note: minimum supported Rust version is 1.74. Windows 10/11 only (Linux coming soon).

## ⚙️ How It Works

1. Define your dev commands in `atomic.toml`.
2. Run them with `atomic <command>`.
3. Atomic runs the command (plus any pre/post hooks).
4. If it changes your Git state, it locally auto-commits.

## 📄 Sample `atomic.toml`

```toml
[custom.check]
command = "cargo check"

[custom.clippy]
before = "echo Running Clippy"
command = "cargo clippy"
after = "echo Clippy finished"

[custom.chain]
command = ["check", "clippy", "cargo fmt"]

[plugin.generate-docs]
script = "./scripts/gen_docs.py"
args = ["hello", "from", "Atomic!"]
silent = true
```

## ✅ Supported Use Cases

- Auto-committing after build/test/lint
- Pre/post hooks around scripts or shell commands
- Cross-platform script runner (Windows PowerShell, batch, Python, etc.)
- Command chaining (`["cargo check", "cargo test"]`)
- Git-integrated plugin system
- Project scaffolding with starter templates

## 🔁 Pre/Post Hooks

Add `before` and `after` to wrap any command:

```toml
[custom.test]
before = "echo Testing..."
command = "cargo test"
after = "echo Done."
```

## 🔌 Plugins

Run script-based plugins (any language):

```toml
[plugin.cleanup]
script = "./scripts/clean_temp.py"
args = ["--force"]
silent = true
```

Call with:

```bash
atomic --plugin cleanup
```

If `silent = true`, logs go to `atomic-logs/cleanup.log`.

## 🧪 Commands

### Atomic Commands

| Command                     | Description                                                |
|-----------------------------|------------------------------------------------------------|
| `atomic init`               | Create an example `atomic.toml` with built-in Rust tasks   |
| `atomic init --template rust` | Use the built-in Rust starter template                  |
| `atomic <command>`          | Run a `[custom.<command>]` entry from your config         |
| `atomic --plugin <name>`    | Run a `[plugin.<name>]` script                            |
| `atomic --list`             | List all available commands from your TOML                |
| `atomic config show`        | Show resolved TOML config in the terminal                 |

> You can also use kebab-case or snake_case for commands — both are supported.

## 🗂 Templates

```bash
atomic init --template rust
atomic init --template example
```

Built-in templates:
- `rust`
- `example`

They include:
- Common Rust commands (check, clippy, test)
- Plugin examples
- Commented TOML

## 💾 Git Auto-Commits

When you run any `custom` or `plugin` command that alters the Git tree, Atomic will auto-commit it **locally** with a timestamp:

```
[2024-03-27 13:52:12] command: cargo fmt
```

You don't have to think about `git add` or `git commit`. It just saves your progress.

## 🔒 Script Support

Works with:
- `.sh`, `.bat`, `.cmd`, `.ps1`, `.py`, `.exe`
- Shell chaining: `&&`, `||`, `;`
- Auto OS detection

## 🛠 Built With

- Rust 1.74
- [clap](https://docs.rs/clap) for CLI parsing
- [git2](https://docs.rs/git2) for native Git integration
- [chrono](https://docs.rs/chrono) for commit timestamps

## 🗺 Roadmap

- [ ] Linux support
- [ ] Plugin chaining (`plugin.build && plugin.deploy`)
- [ ] Template variables with default values (`{{ var | default("fallback") }}`)
- [ ] Git hook integration
- [ ] Command caching or skip-if-clean behavior
- [ ] Web UI (log viewer or dashboard)

Contribute or suggest features on [GitHub](https://github.com/ExtremelyRyan/atomic/issues).

## 🙋 Support

- Ask a [question](https://github.com/ExtremelyRyan/atomic/issues/new?labels=question)
- Request a [feature](https://github.com/ExtremelyRyan/atomic/issues/new?labels=enhancement)
- Report a [bug](https://github.com/ExtremelyRyan/atomic/issues/new?labels=bug)

## ✍️ Author

Built by [ExtremelyRyan](https://github.com/ExtremelyRyan). MIT licensed. Star the repo if you use it!

## 🔗 Project Metadata

- Crate name: `cargo-atomic`
- Binary name: `atomic`
- License: MIT
- Categories: CLI Tools, Git Automation, Rust Dev Utilities
