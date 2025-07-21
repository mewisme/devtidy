## 📦 DevTidy (dd)

> **Clean development artifacts from your projects** — with an interactive terminal UI and AI-powered assistance.

**DevTidy** is a command-line tool with a TUI (Text-based User Interface) that helps you scan and remove unnecessary development files and folders such as `target/`, `node_modules/`, `.log` files, and other build artifacts to free up disk space.

---

### ✨ Features

- **Interactive Terminal UI (TUI)** for easy navigation and selection
- **AI-Powered Cleanup Assistance** with local Ollama models (offline inference)
- **Smart Folder Analysis** and cleanup suggestions
- **Automatic Ollama Management** - starts and stops daemon as needed
- **Cross-platform**: Linux, macOS, Windows
- **Self-installation** with `--install` flag
- **Configurable scan depth** and `.gitignore` support
- **Asynchronous scanning** for improved performance
- **Human-readable file size** display
- **Comprehensive language support** (JavaScript, Rust, Python, Java, C++, etc.)

---

### 🔧 Installation

#### Option 1: Download prebuilt binary

Download the binary from [Releases](https://github.com/mewisme/devtidy/releases), unzip, and place it in your `PATH`.

#### Option 2: Install directly from the executable

```bash
./dd --install
```

This will:
- Copy `dd` to `~/.devtidy/`
- Add that path to your system `PATH` (Windows) or to `.bashrc` / `.zshrc` (Unix)

#### Option 3: Build from source

```bash
cargo build --release
```

The binary will be available at `target/release/dd`.

---

### 🚀 Usage

#### Interactive Mode (TUI)

Run DevTidy in the current directory:
```bash
dd
```

Run DevTidy in a specific directory:
```bash
dd --path /path/to/directory
```

#### Navigation
- Use **arrow keys** to navigate
- Press **Space** to select files/folders
- Press **c** to delete selected items
- Press **h** for help
- Press **q** to quit

#### Advanced Options
```bash
# Scan with .gitignore support
dd --gitignore

# Limit scan depth (default is 6)
dd --depth 4

# Combine options
dd --path ./my_project --gitignore --depth 3
```

---

### 🤖 AI-Powered Commands

DevTidy includes local AI assistance using **Ollama** for intelligent cleanup decisions:

#### Explain Folders/Files
```bash
dd ai-explain node_modules    # Explain specific folder
dd ai-explain                 # Explain current directory
```

#### Get Cleanup Suggestions
```bash
dd ai-suggest                 # AI analysis of current directory
```

#### Interactive AI Chat
```bash
dd ai-chat                    # Start conversation about cleanup
```

#### Automatic Ollama Management
- **Automatically starts** Ollama daemon if not running
- **Downloads models** on first use
- **Cleans up processes** when done
- **Preserves existing** Ollama sessions

#### Smart Hardware-Aware Model Selection
**GPU Detection:**
- **NVIDIA GPUs** (via `nvidia-smi`) → Larger models based on VRAM
- **AMD GPUs** (via `rocm-smi`) → Conservative model selection  
- **Apple Silicon** (M1/M2/M3) → Efficient unified memory usage
- **Intel Graphics** → Small models with shared memory
- **CPU Only** → Conservative model selection

**Model Selection Logic:**
- **NVIDIA RTX 4090/3090** (≥16GB VRAM) → `mistral:instruct`
- **NVIDIA RTX 3080/4070** (8-12GB VRAM) → `gemma:7b`
- **NVIDIA RTX 3060/3070** (4-8GB VRAM) → `gemma:2b`
- **Apple Silicon** (≥16GB RAM) → `mistral:instruct`
- **CPU Only** (≥8GB RAM, 8+ cores) → `gemma:2b`
- **Limited Systems** (≤4GB RAM) → `phi` or `tinyllama`

#### Context-Aware AI Features
- **Pattern Recognition**: AI knows all DevTidy cleanable patterns and provides informed advice
- **Conversation Memory**: AI chat remembers previous exchanges for better context
- **Smart Recommendations**: When asked about files/folders, AI checks against known patterns
- **Streaming Responses**: ALL AI commands use real-time streaming for immediate feedback
  - No waiting for complete responses
  - Text appears as it's generated
  - Better user experience with live updates
  - **Model-specific token limits** prevent truncated responses
  - **Robust error handling** with retry logic and detailed diagnostics

---

### 🛠 CLI Options

| Command | Description |
|---------|-------------|
| `dd` | Start interactive TUI mode |
| `dd ai-explain [path]` | AI explanation of folder/file |
| `dd ai-suggest` | AI cleanup suggestions for current directory |
| `dd ai-chat` | Interactive AI chat for cleanup advice |
| `dd ai-diagnose` | Run AI system diagnostics and troubleshooting |
| `dd ai-test-context` | Test AI context functionality (debug) |

| Option | Description |
|---------|-------------|
| `--path`, `-p` | Target directory to scan (default: current) |
| `--gitignore` | Respect `.gitignore` patterns |
| `--depth`, `-d` | Maximum scan depth (default: 6) |
| `--install`, `-i` | Install `dd` globally |
| `--version`, `-v` | Show version information |
| `--help`, `-h` | Show help information |

---

### 🗂 Supported File Patterns

DevTidy recognizes and can clean:

#### **JavaScript/Node.js**
- `node_modules/`, `dist/`, `coverage/`
- `.next/`, `.turbo/`, `.svelte-kit/`
- `.parcel-cache/`, `.vite/`, `.yarn/`

#### **Rust**
- `target/`, `debug/`, `release/`

#### **Python**
- `__pycache__/`, `.pytest_cache/`, `.mypy_cache/`
- `.ruff_cache/`, `venv/`, `.venv/`, `env/`
- `*.pyc`, `*.pyo`

#### **Java/Kotlin/Gradle**
- `build/`, `.gradle/`, `out/`

#### **C/C++/CMake**
- `cmake-build-debug/`, `cmake-build-release/`
- `build-*/`

#### **Elixir**
- `_build/`

And many more patterns for various development environments.

---

### 🏗 Project Structure

```
src/
├── main.rs           # CLI entry point and argument parsing
├── core/             # Core application logic
│   ├── app.rs        # Main application state and TUI logic
│   ├── models.rs     # Data structures and models
│   └── constants.rs  # Cleanable patterns and constants
├── services/         # Business logic services
│   ├── scanner.rs    # File system scanning logic
│   └── cleaner.rs    # File deletion operations
├── ai/               # AI integration
│   ├── ollama.rs     # Ollama client and model management
│   ├── commands.rs   # AI command handlers
│   └── utils.rs      # AI utilities
└── ui/               # User interface
    └── ui.rs         # TUI rendering and styling
```

---

### ⚙️ Building from Source

```bash
# Standard build
cargo build --release

# Development build with debug info
cargo build

# Run tests
cargo test

# Check code
cargo check
```

---

### 📋 Dependencies

DevTidy is built with modern Rust libraries:

**Core Framework:**
- [clap](https://crates.io/crates/clap) - Command-line argument parsing
- [tokio](https://crates.io/crates/tokio) - Asynchronous runtime
- [anyhow](https://crates.io/crates/anyhow) - Error handling

**TUI Components:**
- [ratatui](https://crates.io/crates/ratatui) - Terminal UI framework
- [crossterm](https://crates.io/crates/crossterm) - Cross-platform terminal

**File Operations:**
- [walkdir](https://crates.io/crates/walkdir) - Directory traversal
- [ignore](https://crates.io/crates/ignore) - `.gitignore` support
- [glob](https://crates.io/crates/glob) - Pattern matching

**AI Integration:**
- [reqwest](https://crates.io/crates/reqwest) - HTTP client for Ollama API
- [serde](https://crates.io/crates/serde) - JSON serialization
- [sysinfo](https://crates.io/crates/sysinfo) - System information for model selection

**Utilities:**
- [human_bytes](https://crates.io/crates/human_bytes) - Human-readable file sizes
- [indicatif](https://crates.io/crates/indicatif) - Progress indicators
- [console](https://crates.io/crates/console) - Terminal styling

---

### 🔧 AI Requirements

To use AI features, you need [Ollama](https://ollama.com) installed:

#### Installation
1. **Automatic**: DevTidy will prompt and guide you through installation
2. **Manual**: Visit [ollama.com/download](https://ollama.com/download)

#### First Run
```bash
# DevTidy automatically:
# 1. Checks if Ollama is installed
# 2. Starts Ollama daemon if needed
# 3. Downloads appropriate model
# 4. Runs inference locally
# 5. Cleans up when done

dd ai-explain
```

---

### 🐛 Troubleshooting

#### AI Commands Not Working
```bash
# Comprehensive AI diagnostics
dd ai-diagnose

# Check if Ollama is installed
ollama --version

# Manually start Ollama (if needed)
ollama serve

# Check available models
ollama list
```

#### Truncated AI Responses (FIXED ✅)
- **Previous issue**: Responses cut off mid-sentence
- **Fix**: Model-specific token limits (tinyllama: 1000, phi: 1500, gemma:2b: 2000 tokens)
- **Result**: Complete responses that finish naturally

#### Timeout Errors (IMPROVED ✅)
- **Previous issue**: `operation timed out` with large models
- **Fix**: Hardware-aware model selection based on GPU/CPU capabilities
- **Result**: Optimal model chosen for your system (CPU Only → tinyllama/phi, NVIDIA GPU → gemma/mistral)

#### Hardware Detection
```bash
# Expected output shows your capabilities:
INFO: Hardware detected - GPU: NVIDIA RTX 3060 (12.0GB VRAM), CPU: 8 cores, RAM: 15.7GB total, 7.5GB available
INFO: Selected model 'gemma:2b' for optimal performance
```

#### TUI Display Issues
- Ensure terminal supports Unicode characters
- Try increasing terminal size
- Check if `TERM` environment variable is set correctly

#### Permission Errors
- Run with appropriate permissions for file deletion
- Check file/folder ownership and permissions

---

### 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

### 📄 License

[MIT License](LICENSE) © 2025 Nguyen Mau Minh

---

### 🙏 Acknowledgments

- Built with ❤️ using Rust
- AI powered by [Ollama](https://ollama.com)
- UI powered by [ratatui](https://ratatui.rs)
- Cross-platform terminal support by [crossterm](https://github.com/crossterm-rs/crossterm)
