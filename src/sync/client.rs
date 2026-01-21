use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipWriter;

/// 获取默认的 skills 目录路径列表（.claude/skills 和 .codex/skills）
fn get_default_skills_dirs() -> Result<Vec<PathBuf>> {
    let home_dir = dirs::home_dir().context("无法获取用户目录")?;
    Ok(vec![
        home_dir.join(".claude").join("skills"),
        home_dir.join(".codex").join("skills"),
    ])
}

/// 扫描目录列表下所有子目录中的 SKILL.md 文件
pub fn scan_skill_files(base_dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut skill_files = Vec::new();

    for base_dir in base_dirs {
        println!("🔍 扫描目录: {}", base_dir.display());

        if !base_dir.exists() {
            println!("⚠️  目录不存在，跳过: {}", base_dir.display());
            continue;
        }

        for entry in WalkDir::new(base_dir)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new("SKILL.md"))
                || path.file_name() == Some(std::ffi::OsStr::new("skill.md"))
            {
                skill_files.push(path.to_path_buf());
            }
        }
    }

    println!("📄 找到 {} 个 SKILL.md 文件", skill_files.len());
    Ok(skill_files)
}

/// 创建包含所有 SKILL.md 的 zip 文件
/// Zip 结构：
///   - skill1.md
///   - skill2.md
///   - ...
///   - manifest.txt (记录每个文件来源：文件名=原始路径)
pub fn create_skills_zip(skill_files: &[PathBuf], zip_path: &Path) -> Result<String> {
    let file = fs::File::create(zip_path).context("创建 zip 文件失败")?;
    let mut zip = ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let pb = ProgressBar::new(skill_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    println!("📦 开始打包 SKILL.md 文件...");

    let mut manifest_lines = Vec::new();
    let mut name_count: HashMap<String, usize> = HashMap::new();

    for skill_file in skill_files {
        pb.set_message(format!("添加: {}", skill_file.display()));

        // 读取文件内容
        let content = fs::read(skill_file).context("读取文件失败")?;

        // 获取技能目录名称作为文件名
        let skill_name = if let Some(parent) = skill_file.parent() {
            parent
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        } else {
            "unknown"
        };

        // 处理重复文件名
        let count = name_count.entry(skill_name.to_string()).or_insert(0);
        let new_filename = if *count == 0 {
            format!("{}.md", skill_name)
        } else {
            format!("{}_{}.md", skill_name, count)
        };
        *count += 1;

        // 添加到 zip 根目录
        zip.start_file(&new_filename, options)?;
        zip.write_all(&content)?;

        // 记录到 manifest，使用正斜杠以支持跨平台
        if let Some(home) = dirs::home_dir() {
            // 使用 Path::strip_prefix 获取相对路径
            let relative = skill_file.strip_prefix(&home).unwrap_or(skill_file);
            // 转换为字符串，统一使用正斜杠
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            manifest_lines.push(format!("{}={}", new_filename, relative_str));
        } else {
            let path_str = skill_file.display().to_string().replace('\\', "/");
            manifest_lines.push(format!("{}={}", new_filename, path_str));
        }

        pb.inc(1);
    }

    // 写入 manifest.txt
    zip.start_file("manifest.txt", options)?;
    for line in &manifest_lines {
        writeln!(zip, "{}", line)?;
    }

    zip.finish()?;
    pb.finish_with_message("打包完成!");

    // 计算 SHA256
    let zip_bytes = fs::read(zip_path)?;
    let hash = Sha256::digest(&zip_bytes);
    Ok(format!("{:x}", hash))
}

/// 上传 zip 文件到远端服务器
pub async fn upload_zip(zip_path: &Path, server_url: &str) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/sync/upload", server_url);

    println!("📤 上传到: {}", url);

    // 获取文件大小用于进度条
    let file_size = fs::metadata(zip_path)?.len();

    let file_content = fs::read(zip_path)?;

    // 创建 multipart form
    let part = reqwest::multipart::Part::bytes(file_content.clone())
        .file_name("skills.zip")
        .mime_str("application/zip")?;

    let form = reqwest::multipart::Form::new().part("file", part);

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.green/white}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );

    println!("⬆️  开始上传...");

    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .context("上传失败")?;

    pb.finish_with_message("上传完成!");

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("上传失败: {} - {}", status, error_text));
    }

    let result: serde_json::Value = response.json().await.context("解析响应失败")?;

    // 提取业务码
    let code = result["body"]["code"]
        .as_str()
        .context("响应中未找到业务码")?;

    Ok(code.to_string())
}

/// 通过业务码下载 zip 文件
pub async fn download_zip(code: &str, server_url: &str, download_path: &Path) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/sync/download/{}", server_url, code);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    pb.set_message("正在下载...");

    let response = client.get(&url).send().await.context("下载请求失败")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("下载失败: {} - {}", status, error_text));
    }

    let bytes = response.bytes().await.context("读取响应内容失败")?;

    // 计算 SHA256
    let hash = Sha256::digest(&bytes);
    let sha256 = format!("{:x}", hash);

    fs::write(download_path, &bytes).context("写入文件失败")?;

    pb.finish_with_message("下载完成!");

    Ok(sha256)
}

/// 解压 zip 文件到目标目录，根据 manifest.txt 恢复原始位置
pub fn extract_zip(zip_path: &Path, _target_dir: &Path) -> Result<()> {
    let file = fs::File::open(zip_path).context("打开 zip 文件失败")?;
    let mut archive = zip::ZipArchive::new(file)?;

    // 先读取 manifest.txt
    let mut manifest_content = String::new();
    let mut file_map: HashMap<String, String> = HashMap::new();

    if let Ok(mut manifest_file) = archive.by_name("manifest.txt") {
        manifest_file.read_to_string(&mut manifest_content)?;
        // 解析 manifest.txt: 文件名=原始路径
        for line in manifest_content.lines() {
            if let Some((filename, original_path)) = line.split_once('=') {
                file_map.insert(filename.to_string(), original_path.to_string());
            }
        }
    }

    // 获取用户目录
    let home_dir = dirs::home_dir().context("无法获取用户目录")?;

    // 重新打开 archive（因为已经读取了 manifest.txt）
    let file = fs::File::open(zip_path).context("打开 zip 文件失败")?;
    let mut archive = zip::ZipArchive::new(file)?;

    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.yellow/white}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;
        let filename = zip_file.name();

        // 跳过 manifest.txt
        if filename == "manifest.txt" {
            pb.inc(1);
            continue;
        }

        pb.set_message(format!("解压: {}", filename));

        // 从 file_map 获取原始路径（包含 SKILL.md）
        if let Some(original_path) = file_map.get(filename) {
            // 路径格式: .codex/skills/humanizer-zh/SKILL.md (已统一为正斜杠)
            // 直接使用这个路径拼接（.claude 和 .codex 中的 . 是目录名的一部分）
            let full_path = home_dir.join(original_path);

            // 检查路径是否已存在且是目录
            if full_path.exists() {
                if full_path.is_dir() {
                    fs::remove_dir_all(&full_path)?;
                } else {
                    fs::remove_file(&full_path)?;
                }
            }

            // 创建父目录
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut outfile = fs::File::create(&full_path)?;
            std::io::copy(&mut zip_file, &mut outfile)?;
        }

        pb.inc(1);
    }

    pb.finish_with_message("解压完成!");

    Ok(())
}

/// 执行上传命令
pub async fn execute_upload(dir: Option<String>, server: String) -> Result<()> {
    let base_dirs = if let Some(d) = dir {
        vec![PathBuf::from(d)]
    } else {
        get_default_skills_dirs()?
    };

    // 创建临时 zip 文件
    let temp_dir = std::env::temp_dir();
    let zip_path = temp_dir.join(format!("skills_{}.zip", chrono::Utc::now().timestamp()));

    // 扫描文件
    let skill_files = scan_skill_files(&base_dirs)?;

    if skill_files.is_empty() {
        println!("❌ 未找到任何 SKILL.md 文件");
        return Ok(());
    }

    // 创建 zip
    let sha256 = create_skills_zip(&skill_files, &zip_path)?;
    println!("✅ Zip 文件 SHA256: {}", sha256);

    // 上传
    let code = upload_zip(&zip_path, &server).await?;
    println!("✅ 业务码: {}", code);

    // 清理临时文件
    fs::remove_file(&zip_path)?;
    println!("🗑️  已清理临时文件");

    Ok(())
}

/// 执行下载命令
pub async fn execute_download(code: String, dir: Option<String>, server: String) -> Result<()> {
    let target_dir = if let Some(d) = dir {
        PathBuf::from(d)
    } else {
        // 默认解压到 .claude/skills
        let home_dir = dirs::home_dir().context("无法获取用户目录")?;
        home_dir.join(".claude").join("skills")
    };

    // 创建临时 zip 文件
    let temp_dir = std::env::temp_dir();
    let zip_path = temp_dir.join(format!("skills_{}.zip", chrono::Utc::now().timestamp()));

    // 下载
    let sha256 = download_zip(&code, &server, &zip_path).await?;
    println!("Zip 文件 SHA256: {}", sha256);

    // 解压
    extract_zip(&zip_path, &target_dir)?;

    // 清理临时文件
    fs::remove_file(&zip_path)?;

    Ok(())
}
