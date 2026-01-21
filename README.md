# Skills Sync

一个用于同步 Claude Code skills 的命令行工具，支持将本地 skills 上传到远端服务器或从服务器下载。

## 功能特性

- **上传 skills**: 扫描本地 skills 目录，打包上传到远端服务器
- **下载 skills**: 通过业务码从服务器下载并恢复 skills 到本地
- **进度显示**: 上传和下载过程显示进度条
- **哈希校验**: 支持 SHA256 哈希值计算确保文件完整性

## 安装

### 从源码编译

```bash
git clone https://github.com/your-username/skills-sync.git
cd skills-sync
cargo build --release
```

编译后的可执行文件位于 `target/release/skills-sync.exe` (Windows) 或 `target/release/skills-sync` (Linux/macOS)。

## 使用方法

### 上传 skills

上传默认目录（`~/.claude/skills/` 和 `~/.codex/skills/`）中的所有 skills：

```bash
skills-sync upload
```

指定上传目录：

```bash
skills-sync upload -d /path/to/skills
```

指定服务器地址：

```bash
skills-sync upload -s http://localhost:8080
```

完整参数示例：

```bash
skills-sync upload -s http://localhost:8080 -d /path/to/skills
```

### 下载 skills

通过业务码下载 skills：

```bash
skills-sync download -c ABC123
```

指定服务器地址：

```bash
skills-sync download -c ABC123 -s http://localhost:8080
```

指定解压目录：

```bash
skills-sync download -c ABC123 -o /path/to/output
```
