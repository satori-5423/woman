<h1 align="center">woman — AI 驱动的手册页翻译工具</h1>

<p align="center"><a href="README.md">English</a> | <strong>中文</strong></p>

**woman** 是 `man` 命令的透明替代品，能够利用大语言模型自动将英文手册页翻译成您的母语。它在非英语语言环境下自动拦截 `man` 查询，通过 LLM API 翻译原始手册页源码，缓存结果并展示——全程保留原始手册页的格式、章节结构和 groff 宏。

## 功能特性

- **🌍 语言环境感知**：检测系统语言环境，仅在非英语环境时进行翻译
- **🤖 AI 驱动翻译**：使用 LLM API 进行高质量的技术文档翻译
- **⚡ 缓存机制**：翻译结果以 gzip 格式缓存，重复查询即时返回
- **🔧 智能格式化**：自动插入 `.na`（不调整）指令，防止 groff 在 CJK 文本中添加额外空格
- **📏 CJK 自动换行**：在 CJK 标点符号处自动断开过长行
- **🐟 Fish Shell 补全**：附带 fish shell 的命令补全文件
- **🔌 可扩展模型支持**：插件化的模型提供商架构（内置 DeepSeek，易于扩展）

## 安装

### 前置依赖

- `cargo` 命令（安装 cargo 或 rustup）
- `man` 命令（man-db 或同类工具）
- 兼容 LLM 提供商的 API 密钥

### 从源码构建

```bash
git clone https://github.com/satori-5423/woman.git
cd woman
cargo build --release
ln -s target/release/woman ~/.local/bin/woman
```

### Shell 补全（Fish）

```bash
cp completions/woman.fish ~/.config/fish/completions/
```

## 配置

woman 的配置文件位于 `~/.local/share/woman/config.json`。

### 设置模型和 API 密钥

```bash
# 列出可用模型
woman model list

# 设置激活模型
woman model set-model deepseek-v4-flash

# 设置 API 密钥
woman model set-key your-api-key-here

# （可选）设置自定义 API 地址
woman model set-url https://api.deepseek.com/v1
```

### 查看当前配置

```bash
woman model show
```

示例输出：

```
Active Model: deepseek-v4-flash
API URL:      https://api.deepseek.com/v1
API Key:      sk-••••••••abcd
```

## 开发

### 运行测试

```bash
cargo test
```

### 添加新的模型提供商

1. 创建 `src/models/<提供商>.rs`
2. 实现 `ModelProvider` trait
3. 在 `src/models/mod.rs` 中注册
4. 将提供商名称添加到 `list_providers()` 中

## 许可证

<p><a href="LICENSE">MIT 许可证</a></p>