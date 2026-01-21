use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipWriter;

/// 获取默认的 skills 目录路径列表（.claude/skills 和 .codex/skills）
fn get_default_skills_dirs() -> Result<Vec<PathBuf>> {
    let home_dir = dirs::home_dir().context("Failed to get home directory / 无法获取用户目录")?;
    Ok(vec![
        home_dir.join(".claude").join("skills"),
        home_dir.join(".codex").join("skills"),
    ])
}

/// 扫描目录列表下所有子目录中的 SKILL.md 文件
pub fn scan_skill_files(base_dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut skill_files = Vec::new();

    for base_dir in base_dirs {
        println!("🔍 Scanning directory / 扫描目录: {}", base_dir.display());

        if !base_dir.exists() {
            println!("⚠️  Directory not found, skipping / 目录不存在，跳过: {}", base_dir.display());
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

    println!("📄 Found {} SKILL.md files / 找到 {} 个 SKILL.md 文件", skill_files.len(), skill_files.len());
    Ok(skill_files)
}

/// 创建包含所有 SKILL.md 的 zip 文件
/// Zip 结构：
///   - skill1.md
///   - skill2.md
///   - ...
///   - manifest.txt (记录每个文件来源：文件名=原始路径)
pub fn create_skills_zip(skill_files: &[PathBuf], zip_path: &Path) -> Result<String> {
    let file = fs::File::create(zip_path).context("Failed to create zip file / 创建 zip 文件失败")?;
    let mut zip = ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let pb = ProgressBar::new(skill_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    println!("📦 Starting to package SKILL.md files / 开始打包 SKILL.md 文件...");

    let mut manifest_lines = Vec::new();
    let mut name_count: HashMap<String, usize> = HashMap::new();
    let mut packaged_files = Vec::new();

    for skill_file in skill_files {
        pb.set_message(format!("Adding / 添加: {}", skill_file.display()));

        // 读取文件内容
        let content = fs::read(skill_file).context("Failed to read file / 读取文件失败")?;

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
            packaged_files.push(format!("~/{}", relative_str));
        } else {
            let path_str = skill_file.display().to_string().replace('\\', "/");
            manifest_lines.push(format!("{}={}", new_filename, path_str));
            packaged_files.push(path_str);
        }

        pb.inc(1);
    }

    // 写入 manifest.txt
    zip.start_file("manifest.txt", options)?;
    for line in &manifest_lines {
        writeln!(zip, "{}", line)?;
    }

    zip.finish()?;
    pb.finish_with_message("Packaging complete / 打包完成!");

    // 显示打包的文件列表
    if !packaged_files.is_empty() {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Packaged files / 打包文件:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for file in &packaged_files {
            println!("  ✓ {}", file);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    // 计算 SHA256
    let zip_bytes = fs::read(zip_path)?;
    let hash = Sha256::digest(&zip_bytes);
    Ok(format!("{:x}", hash))
}

/// 上传 zip 文件到远端服务器
pub async fn upload_zip(zip_path: &Path, server_url: &str) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/sync/upload", server_url);

    println!("📤 Uploading to / 上传到: {}", url);

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
            .template("[{elapsed_precise}] [{bar:40.green/white}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("=>-"),
    );

    println!("⬆️  Starting upload / 开始上传...");

    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .context("Upload failed / 上传失败")?;

    pb.finish_with_message("Upload complete / 上传完成!");

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Upload failed / 上传失败: {} - {}", status, error_text));
    }

    let result: serde_json::Value = response.json().await.context("Failed to parse response / 解析响应失败")?;

    // 提取业务码
    let code = result["body"]["code"]
        .as_str()
        .context("Business code not found in response / 响应中未找到业务码")?;

    Ok(code.to_string())
}

/// 通过业务码下载 zip 文件
pub async fn download_zip(code: &str, server_url: &str, download_path: &Path) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/sync/download/{}", server_url, code);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} [{elapsed_precise}] {msg}")?,
    );
    pb.set_message("Downloading / 正在下载...");

    let response = client.get(&url).send().await.context("Download request failed / 下载请求失败")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Download failed / 下载失败: {} - {}", status, error_text));
    }

    let bytes = response.bytes().await.context("Failed to read response / 读取响应内容失败")?;

    // 计算 SHA256
    let hash = Sha256::digest(&bytes);
    let sha256 = format!("{:x}", hash);

    fs::write(download_path, &bytes).context("Failed to write file / 写入文件失败")?;

    pb.finish_with_message("Download complete / 下载完成!");

    Ok(sha256)
}

/// 解压 zip 文件到目标目录，根据 manifest.txt 恢复原始位置
pub fn extract_zip(zip_path: &Path, _target_dir: &Path) -> Result<()> {
    let file = fs::File::open(zip_path).context("Failed to open zip file / 打开 zip 文件失败")?;
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
    let home_dir = dirs::home_dir().context("Failed to get home directory / 无法获取用户目录")?;

    // 重新打开 archive（因为已经读取了 manifest.txt）
    let file = fs::File::open(zip_path).context("Failed to open zip file / 打开 zip 文件失败")?;
    let mut archive = zip::ZipArchive::new(file)?;

    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.yellow/white}] {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    // 记录解压的文件
    let mut extracted_files = Vec::new();

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;
        let filename = zip_file.name();

        // 跳过 manifest.txt
        if filename == "manifest.txt" {
            pb.inc(1);
            continue;
        }

        pb.set_message(format!("Extracting / 解压: {}", filename));

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

            // 记录解压的文件
            extracted_files.push(format!("~/{}", original_path));
        }

        pb.inc(1);
    }

    pb.finish_with_message("Extraction complete / 解压完成!");

    // 显示解压的文件列表
    if !extracted_files.is_empty() {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Extracted files / 解压文件:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for file in &extracted_files {
            println!("  ✓ {}", file);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

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
        println!("❌ No SKILL.md files found / 未找到任何 SKILL.md 文件");
        return Ok(());
    }

    // 创建 zip
    let sha256 = create_skills_zip(&skill_files, &zip_path)?;
    println!("✅ Zip file SHA256 / Zip 文件 SHA256: {}", sha256);

    // 上传
    let code = upload_zip(&zip_path, &server).await?;
    println!("✅ Business code / 业务码: {}", code);

    // 清理临时文件
    fs::remove_file(&zip_path)?;
    println!("🗑️  Temporary files cleaned / 已清理临时文件");

    Ok(())
}

/// 执行下载命令
pub async fn execute_download(code: String, dir: Option<String>, server: String) -> Result<()> {
    let target_dir = if let Some(d) = dir {
        PathBuf::from(d)
    } else {
        // 默认解压到 .claude/skills
        let home_dir = dirs::home_dir().context("Failed to get home directory / 无法获取用户目录")?;
        home_dir.join(".claude").join("skills")
    };

    // 创建临时 zip 文件
    let temp_dir = std::env::temp_dir();
    let zip_path = temp_dir.join(format!("skills_{}.zip", chrono::Utc::now().timestamp()));

    // 下载
    let sha256 = download_zip(&code, &server, &zip_path).await?;
    println!("Zip file SHA256 / Zip 文件 SHA256: {}", sha256);

    // 解压
    extract_zip(&zip_path, &target_dir)?;

    // 清理临时文件
    fs::remove_file(&zip_path)?;

    Ok(())
}

/// Skill 信息结构体
struct SkillInfo {
    name: String,
    description: String,
    path: String,
}

/// SKILL.md 的 YAML front matter 结构
#[derive(Deserialize)]
struct SkillMetadata {
    name: Option<String>,
    description: Option<String>,
    #[serde(rename = "allowed-tools")]
    allowed_tools: Option<Vec<String>>,
    metadata: Option<serde_yaml::Value>,
}

/// 从 SKILL.md 文件中提取描述信息
fn extract_description(content: &str) -> String {
    // 提取 YAML front matter (--- 之间的内容)
    if let Some(yaml_start) = content.find("---") {
        if let Some(yaml_end) = content[yaml_start + 3..].find("---") {
            let yaml_content = &content[yaml_start + 3..yaml_start + 3 + yaml_end];

            // 使用 serde_yaml 反序列化
            if let Ok(metadata) = serde_yaml::from_str::<SkillMetadata>(yaml_content) {
                if let Some(desc) = metadata.description {
                    // 清理换行符和多余空格
                    let cleaned = desc
                        .lines()
                        .map(|line| line.trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                    return cleaned.chars().take(100).collect::<String>();
                }
            }
        }
    }

    // 如果没有找到 YAML description，尝试其他格式
    let patterns = [
        // 匹配 ## Description / ## 描述 部分
        "##\\s*(?:Description|描述)\\s*\\n\\s*([^\\n]+)",
        // 匹配 [!description]: ... 格式
        "\\[!?description\\]:\\s*([^\\n]+)",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(content) {
                if let Some(desc) = caps.get(1) {
                    return desc.as_str().trim().to_string();
                }
            }
        }
    }

    // 如果没有找到特定的描述字段，尝试提取第一段非空文本
    for line in content.lines() {
        let trimmed = line.trim();
        // 跳过 YAML 相关行、标题行和空行
        if !trimmed.starts_with('#')
            && !trimmed.starts_with("---")
            && !trimmed.starts_with("name:")
            && !trimmed.starts_with("description:")
            && !trimmed.starts_with("allowed-tools:")
            && !trimmed.starts_with("metadata:")
            && !trimmed.is_empty()
        {
            return trimmed.chars().take(80).collect::<String>();
        }
    }

    "No description".to_string()
}

/// 执行列表命令
pub fn execute_list(dir: Option<String>) -> Result<()> {
    let base_dirs = if let Some(d) = dir {
        vec![PathBuf::from(d)]
    } else {
        get_default_skills_dirs()?
    };

    // 按来源目录分组存储 skills
    let mut skills_by_source: Vec<(String, Vec<SkillInfo>)> = Vec::new();

    for base_dir in &base_dirs {
        let mut skills = Vec::new();

        if !base_dir.exists() {
            continue;
        }

        // 确定来源名称
        let source_name = if let Some(parent) = base_dir.parent() {
            parent
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            "Unknown".to_string()
        };

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
                // 获取 skill 名称（目录名）
                let name = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // 读取文件内容
                let content = fs::read_to_string(path).unwrap_or_default();
                let description = extract_description(&content);

                // 获取相对路径
                let home_dir = dirs::home_dir().context("Failed to get home directory / 无法获取用户目录")?;
                let relative_path = path
                    .strip_prefix(&home_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");

                skills.push(SkillInfo {
                    name,
                    description,
                    path: format!("~/{}", relative_path),
                });
            }
        }

        if !skills.is_empty() {
            skills_by_source.push((source_name, skills));
        }
    }

    if skills_by_source.is_empty() {
        println!("❌ No skills found / 未找到任何 skills");
        return Ok(());
    }

    let total_count: usize = skills_by_source.iter().map(|(_, v)| v.len()).sum();

    // 按来源分组显示
    for (source, skills) in &skills_by_source {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  📁 {} directory / {} 目录 - {} skills",
                 source, source, skills.len());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // 创建表格
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                "Name / 名称",
                "Description / 描述",
                "Path / 路径",
            ]);

        for skill in skills {
            table.add_row(vec![
                skill.name.as_str(),
                skill.description.as_str(),
                skill.path.as_str(),
            ]);
        }

        println!("{table}");
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Total / 总计: {} skills", total_count);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    Ok(())
}
