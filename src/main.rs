mod sync;

use crate::sync::cli::Cli;
use crate::sync::client::{execute_download, execute_list, execute_upload};
use clap::Parser;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let cli = Cli::parse();

    if let Err(e) = run_sync_client(cli).await {
        eprintln!("❌ 错误: {}", e);
        std::process::exit(1);
    }
}

async fn run_sync_client(cli: Cli) -> Result<(), anyhow::Error> {
    match cli.command {
        crate::sync::cli::Command::Upload { dir } => {
            execute_upload(dir, cli.server).await?;
        }
        crate::sync::cli::Command::Download { code, dir } => {
            execute_download(code, dir, cli.server).await?;
        }
        crate::sync::cli::Command::List { dir } => {
            execute_list(dir)?;
        }
    }
    Ok(())
}
