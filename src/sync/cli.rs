use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "skills-sync")]
#[command(about = "Skills 同步工具 - 上传和下载本地 skills 到远端仓库")]
#[command(version)]
#[command(after_help = r#"
EXAMPLES / 示例:
  Upload skills to default server / 上传 skills 到默认服务器:
    cargo run -- upload

  Upload to specified server / 上传到指定服务器:
    cargo run -- upload -s http://localhost:8080

  Download from server / 从服务器下载:
    cargo run -- download -c ABC123

  List locally installed skills / 列出本地已安装的 skills:
    cargo run -- list

DEFAULT SCAN DIRECTORIES / 默认扫描目录:
  ~/.claude/skills/
  ~/.codex/skills/
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// 远端服务器地址 (未指定时根据 IP 自动选择 / Auto-selected by IP if not specified)
    #[arg(
        short = 's',
        long,
        global = true
    )]
    pub server: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 上传本地 skills 到远端仓库 / Upload local skills to remote repository
    Upload {
        /// 本地 skills 目录路径 / Local skills directory path
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },

    /// 从远端仓库下载 skills / Download skills from remote repository
    Download {
        /// 业务码 / Business code
        #[arg(short = 'c', long)]
        code: String,

        /// 解压目标目录 / Extract target directory
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },

    /// 列出本地已安装的 skills / List locally installed skills
    List {
        /// 本地 skills 目录路径 / Local skills directory path
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },
}
