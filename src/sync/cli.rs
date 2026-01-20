use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "skills-sync")]
#[command(about = "Skills 同步工具 - 上传和下载本地 skills 到远端仓库", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// 远端服务器地址
    #[arg(short = 's', long, global = true, default_value = "http://localhost:9090")]
    pub server: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 上传本地 skills 到远端仓库
    Upload {
        /// 本地 skills 目录路径（默认为用户目录下的 .claude/.codex/skills）
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },

    /// 从远端仓库下载 skills
    Download {
        /// 业务码
        #[arg(short = 'c', long)]
        code: String,

        /// 解压目标目录（默认为用户目录下的 .claude/.codex/skills）
        #[arg(short = 'd', long)]
        dir: Option<String>,
    },
}
