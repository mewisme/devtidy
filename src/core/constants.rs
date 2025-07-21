use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static CLEANABLE_PATTERNS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  let mut patterns = HashMap::new();

  // ───── JavaScript / Node.js ─────
  patterns.insert("node_modules", "Node.js dependencies");
  patterns.insert("pnpm-lock.yaml", "pnpm lock file");
  patterns.insert(".yarn", "Yarn cache directory");
  patterns.insert(".parcel-cache", "Parcel bundler cache");
  patterns.insert(".next", "Next.js build artifacts");
  patterns.insert(".turbo", "Turborepo build artifacts");
  patterns.insert(".svelte-kit", "SvelteKit build artifacts");
  patterns.insert(".vite", "Vite cache directory");
  patterns.insert("dist", "Distribution files");
  patterns.insert("coverage", "Test coverage reports");
  patterns.insert("node_modules/.cache", "npm/yarn/pnpm internal cache");

  // ───── Rust ─────
  patterns.insert("target", "Rust build artifacts");
  patterns.insert("debug", "Rust debug output");
  patterns.insert("release", "Rust release output");
  patterns.insert("deps", "Rust/Elixir dependencies");

  // ───── Python ─────
  patterns.insert("__pycache__", "Python bytecode cache");
  patterns.insert(".pytest_cache", "Pytest cache");
  patterns.insert(".mypy_cache", "MyPy static analysis cache");
  patterns.insert(".ruff_cache", "Ruff linter cache");
  patterns.insert("venv", "Python virtual environment");
  patterns.insert(".venv", "Python virtual environment");
  patterns.insert("env", "Python virtual environment");
  patterns.insert("*.pyc", "Compiled Python files");
  patterns.insert("*.pyo", "Optimized Python files");

  // ───── Elixir ─────
  patterns.insert("_build", "Elixir build artifacts");

  // ───── Java / Kotlin / Gradle ─────
  patterns.insert("build", "Build output directory");
  patterns.insert(".gradle", "Gradle build cache");
  patterns.insert("out", "Output directory");

  // ───── C / C++ / CMake ─────
  patterns.insert("cmake-build-debug", "CMake debug build artifacts");
  patterns.insert("cmake-build-release", "CMake release build artifacts");
  patterns.insert("build-*", "Wildcard build output directories");

  // ───── macOS / iOS / Xcode ─────
  patterns.insert("DerivedData", "Xcode derived data");
  patterns.insert(".DS_Store", "macOS metadata");

  // ───── Editor / IDE configs ─────
  patterns.insert(".vscode", "VS Code configuration");
  patterns.insert(".idea", "JetBrains IDE configuration");

  // ───── General build or tool cache ─────
  patterns.insert(".cache", "Generic build cache");
  patterns.insert(".scannerwork", "SonarQube scanner cache");

  // ───── Miscellaneous files ─────
  patterns.insert("*.log", "Log files");
  patterns.insert("*.tmp", "Temporary files");
  patterns.insert("*.bak", "Backup files");
  patterns.insert("*.old", "Old backup files");
  patterns.insert("*.swp", "Vim swap files");
  patterns.insert("*.swo", "Vim swap files");
  patterns.insert(".env", "Environment variable file");
  patterns.insert("docker-compose.override.yml", "Docker override config");
  patterns.insert("*.db", "Database files");
  patterns.insert("*.sqlite3", "SQLite database files");

  patterns
});
