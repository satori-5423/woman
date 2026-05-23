<h1 align="center">woman — AI-Powered Man Page Translation Tool</h1>

<p align="center"><strong>English</strong> | <a href="README.zh-CN.md">中文</a></p>

**woman** is a transparent drop-in replacement for the `man` command. It leverages Large Language Models (LLMs) to automatically translate English man pages into your native language. When run in a non-English locale, it intercepts `man` queries, translates the raw man page source code via an LLM API, and caches the results for display—all while preserving the original layout, section structure, and groff macros.

## Features

- **🌍 Locale-Aware**: Detects the system locale and only triggers translation when in non-English environments.
- **🤖 AI-Powered**: Uses LLM APIs to deliver high-quality technical translations.
- **⚡ Caching Mechanism**: Caches translation results in gzip format to ensure subsequent queries load instantly.
- **🔧 Smart Formatting**: Automatically inserts `.na` (no adjust) directives to prevent groff from adding awkward extra spaces in CJK text.
- **📏 CJK Line Wrapping**: Automatically wraps excessively long lines at CJK punctuation marks.
- **🐟 Fish Shell Completions**: Includes a command-line completion file for the Fish shell.
- **🔌 Extensible Model Support**: Pluggable architecture for model providers (built-in support for DeepSeek, easy to extend).

## Installation

### Prerequisites

- `cargo` command (install cargo or rustup)
- `man` command (man-db or equivalent)
- An API key for a compatible LLM provider

### Build from Source

```bash
git clone https://github.com/satori-5423/woman.git
cd woman
cargo build --release
ln -s target/release/woman ~/.local/bin/woman
```

### Shell Completions (Fish)

```bash
cp completions/woman.fish ~/.config/fish/completions/
```

## Configuration

The configuration file for woman is located at `~/.local/share/woman/config.json`.

### Set Model and API Key

```bash
# List available models
woman model list

# Set the active model
woman model set-model deepseek-v4-flash

# Set the API key
woman model set-key your-api-key-here

# (Optional) Set a custom API URL
woman model set-url https://api.deepseek.com/v1
```

### View Current Configuration

```bash
woman model show
```

Example output:

```
Active Model: deepseek-v4-flash
API URL:      https://api.deepseek.com/v1
API Key:      sk-••••••••abcd
```

## Development

### Running Tests

```bash
cargo test
```

### Adding a New Model Provider

1. Create `src/models/<provider>.rs`
2. Implement the `ModelProvider` trait
3. Register the provider in `src/models/mod.rs`
4. Add the provider name to `list_providers()`

## License

<p><a href="LICENSE">MIT License</a></p>