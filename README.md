## 📦 DevTidy (dd)

> **Clean development artifacts from your projects** — with an interactive terminal UI.

**DevTidy** is a command-line tool with a TUI (Text-based User Interface) that helps you scan and remove unnecessary development files and folders such as `target/`, `node_modules/`, `.log` files, and other build artifacts to free up disk space.

---

### ✨ Features

* Interactive terminal UI (TUI) for easy navigation and selection
* `.gitignore` support to skip ignored files
* Cross-platform: **Linux, macOS, Windows**
* Self-installation with `--install` flag
* Configurable scan depth
* Asynchronous scanning for improved performance
* Human-readable file size display

---

### 🔧 Installation

#### Option 1: Download prebuilt binary

Download the binary from [Releases](https://github.com/mewisme/devtidy/releases), unzip, and place it in your `PATH`.

#### Option 2: Install directly from the executable

```bash
./dd --install
```

This will:

* Copy `dd` to `~/.devtidy/`
* Add that path to your system `PATH` (Windows) or to `.bashrc` / `.zshrc` (Unix)

#### Option 3: Build from source

```bash
cargo build --release
```

The binary will be available at `target/release/dd`.

---

### 🚀 Usage

Run DevTidy in the current directory:

```bash
dd
```

Run DevTidy in a specific directory:

```bash
dd /path/to/directory
```

### Navigation

* Use arrow keys to navigate
* Press `Space` to select files/folders
* Press `c` to delete selected items
* Press `q` to quit

### Advanced Usage

Scan with `.gitignore` support:

```bash
dd ./my_project --gitignore
```

Limit scan depth (default is 6):

```bash
dd --depth 4
```

---

### 🛠 CLI Options

| Option            | Description                             |
| ----------------- | --------------------------------------- |
| `--gitignore`     | Ignore files listed in `.gitignore`     |
| `--depth`, `-d`   | Maximum depth for scanning (default: 6) |
| `--install`, `-i` | Install `dd` globally                  |
| `--version`, `-v` | Show version info                       |

---

### ⚙️ Building from source

```bash
# Standard build
cargo build --release

# Cross-compile
cargo build --release --target x86_64-unknown-linux-gnu
```

### Dependencies

DevTidy is built with:

- [ratatui](https://crates.io/crates/ratatui) - Terminal UI library
- [crossterm](https://crates.io/crates/crossterm) - Terminal manipulation
- [walkdir](https://crates.io/crates/walkdir) & [ignore](https://crates.io/crates/ignore) - File system traversal
- [tokio](https://crates.io/crates/tokio) - Asynchronous runtime

---

### 📄 License

[MIT License](LICENSE) © 2025 Nguyen Mau Minh
