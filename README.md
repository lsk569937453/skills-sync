# Skills Sync

A command-line tool for synchronizing Claude Code skills. Upload local skills to a remote server or download skills from the server.

## Features

- **Upload skills**: Scan local skills directories, package and upload to remote server
- **Download skills**: Download and restore skills from server using business code
- **Progress display**: Progress bars for upload and download operations
- **Hash verification**: SHA256 hash calculation to ensure file integrity
- **List skills**: Display locally installed skills in a table format

## Installation

### Build from source

```bash
git clone https://github.com/your-username/skills-sync.git
cd skills-sync
cargo build --release
```

The compiled executable will be located at `target/release/skills-sync.exe` (Windows) or `target/release/skills-sync` (Linux/macOS).

## Usage

### Upload skills

Upload all skills from default directories (`~/.claude/skills/` and `~/.codex/skills/`):

```bash
skills-sync upload
```

Upload from a specific directory:

```bash
skills-sync upload -d /path/to/skills
```

Specify server address:

```bash
skills-sync upload -s http://localhost:8080
```

Full Example / 完整参数示例:

```bash
skills-sync upload -s http://localhost:8080 -d /path/to/skills
```

### Download skills

Download skills using business code:

```bash
skills-sync download -c ABC123
```

Specify server address:

```bash
skills-sync download -c ABC123 -s http://localhost:8080
```

Specify extraction directory:

```bash
skills-sync download -c ABC123 -d /path/to/output
```

### List skills

List all locally installed skills:

```bash
skills-sync list
```

List skills from a specific directory:

```bash
skills-sync list -d /path/to/skills
```

## Default Scan Directories

- `~/.claude/skills/`
- `~/.codex/skills/`

## Commands

| Command | Description |
|---------|-------------|
| `upload` | Upload local skills to remote repository |
| `download` | Download skills from remote repository |
| `list` | List locally installed skills |

## Options

| Option | Description |
|--------|-------------|
| `-s, --server <URL>` | Remote server address (default: `https://www.937453.xyz`) |
| `-d, --dir <PATH>` | Local skills directory path |
| `-c, --code <CODE>` | Business code (for download) |
| `-h, --help` | Display help information |
| `-V, --version` | Display version information |

## Output Examples

### Upload

```bash
$ skills-sync upload
🔍 Scanning directory / 扫描目录: C:\Users\user\.claude\skills
🔍 Scanning directory / 扫描目录: C:\Users\user\.codex\skills
📄 Found 4 SKILL.md files / 找到 4 个 SKILL.md 文件
📦 Starting to package SKILL.md files / 开始打包 SKILL.md 文件...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Packaged files / 打包文件:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✓ ~/.claude/skills/humanizer-zh/SKILL.md
  ✓ ~/.claude/skills/vercel-react-best-practices/SKILL.md
  ✓ ~/.codex/skills/humanizer-zh/SKILL.md
  ✓ ~/.codex/skills/vercel-react-best-practices/SKILL.md
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Zip file SHA256 / Zip 文件 SHA256: c07f91bf155a0b0669a0928db0b5e909fc3204bb92e5101465a39c5378b8d5b6
📤 Uploading to / 上传到: https://www.937453.xyz/sync/upload
⬆️  Starting upload / 开始上传...
✅ Business code / 业务码: 4966f452-7365-4b2b-a218-6f0736976777
🗑️  Temporary files cleaned / 已清理临时文件
```

### Download

```bash
$ skills-sync download -c ABC123
Downloading / 正在下载...
Download complete / 下载完成!
Zip file SHA256 / Zip 文件 SHA256: c19544cf7fd5872d08d75bf1b3207c279908bd25f14e8216808c86a64f98fc95

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Extracted files / 解压文件:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✓ ~/.claude/skills/humanizer-zh/SKILL.md
  ✓ ~/.claude/skills/vercel-react-best-practices/SKILL.md
  ✓ ~/.codex/skills/humanizer-zh/SKILL.md
  ✓ ~/.codex/skills/vercel-react-best-practices/SKILL.md
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### List

```bash
$ skills-sync list

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  📁 .claude directory / .claude 目录 - 2 skills
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌─────────────────────────────┬─────────────────────┬────────────────────────────────┐
│ Name / 名称                 ┆ Description / 描述  ┆ Path / 路径                    │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ humanizer-zh                ┆ Remove AI writing... ┆ ~/.claude/skills/.../SKILL.md │
└─────────────────────────────┴─────────────────────┴────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Total / 总计: 4 skills
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## License

MIT
