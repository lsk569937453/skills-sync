use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "skills-sync")]
#[command(about = "Skills 同步工具 - 上传和下载本地 skills 到远端仓库")]
#[command(version)]
#[command(after_help = r#"
示例:
  上传 skills 到默认服务器:
    cargo run -- upload

  上传到指定服务器:
    cargo run -- upload -s http://localhost:8080

  从服务器下载:
    cargo run -- download -c ABC123

  指定目录上传:
    cargo run -- upload -d "C:\path\to\skills"

默认扫描目录:
  ~/.claude/skills/
  ~/.codex/skills/
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// 远端服务器地址
    #[arg(
        short = 's',
        long,
        global = true,
        default_value = "https://www.937453.xyz"
    )]
    pub server: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 上传本地 skills 到远端仓库
    Upload {
        /// 本地 skills 目录路径
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },

    /// 从远端仓库下载 skills
    Download {
        /// 业务码
        #[arg(short = 'c', long)]
        code: String,

        /// 解压目标目录
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },
}
