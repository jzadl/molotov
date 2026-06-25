use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::Path;
use std::process::Command;

use mltv::parser::Parser;
use mltv::tokenizer::tokenize;
use mltv::transpiler::transpile_with_dir;

#[derive(ClapParser)]
#[command(
    name = "mltv",
    version = "1.1",
    about = "Molotov programming language compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the .mltv source file to run (shorthand for 'run' command)
    file: Option<String>,

    /// Arguments to pass to the script
    #[arg(last = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Transpile and compile a .mltv file into a Rust binary
    Deploy {
        /// Path to the .mltv source file
        file: String,
        /// Output binary name (optional)
        #[arg(short, long)]
        output: Option<String>,
        /// Keep the generated .rs file
        #[arg(long)]
        keep: bool,
        /// Only transpile to Rust, do not compile
        #[arg(long)]
        rust_only: bool,
    },
    /// Transpile, compile, and run a .mltv file immediately
    Run {
        /// Path to the .mltv source file
        file: String,
        /// Arguments to pass to the script
        #[arg(last = true)]
        args: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Deploy {
            file,
            output,
            keep,
            rust_only,
        }) => {
            deploy(&file, output.as_deref(), keep, rust_only)?;
        }
        Some(Commands::Run { file, args }) => {
            run_file(&file, &args)?;
        }
        None => {
            if let Some(file) = cli.file {
                run_file(&file, &cli.args)?;
            } else {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
            }
        }
    }

    Ok(())
}

fn deploy(file: &str, output: Option<&str>, keep: bool, rust_only: bool) -> anyhow::Result<()> {
    let source_path = Path::new(file);
    if !source_path.exists() {
        anyhow::bail!("file '{}' not found", file);
    }

    let source = fs::read_to_string(source_path)
        .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", file, e))?;

    let tokens = tokenize(&source).map_err(|e| anyhow::anyhow!("tokenization error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| anyhow::anyhow!("parse error: {}", e))?;

    let source_dir = source_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let rust_code = transpile_with_dir(&program, &source_dir)
        .map_err(|e| anyhow::anyhow!("transpilation error: {}", e))?;

    let output_name = output.unwrap_or("a.out");
    let rs_path = Path::new(output_name).with_extension("rs");

    fs::write(&rs_path, &rust_code)
        .map_err(|e| anyhow::anyhow!("failed to write '{}': {}", rs_path.display(), e))?;

    if rust_only {
        println!("Transpiled to {}", rs_path.display());
        return Ok(());
    }

    let binary_name = if output_name.ends_with(".rs") {
        output_name.trim_end_matches(".rs")
    } else {
        output_name
    };

    // Try cargo first, fall back to rustc
    let deploy_dir = rs_path.parent().unwrap_or(Path::new("."));
    let cargo_toml = r#"[package]
name = "mltv_deploy"
version = "0.1.0"
edition = "2021"

[dependencies]
serde_json = "1"
rand = "0.8"
"#;
    let _ = fs::write(deploy_dir.join("Cargo.toml"), cargo_toml);

    let src_dir = deploy_dir.join("src");
    let _ = fs::create_dir_all(&src_dir);
    let _ = fs::copy(&rs_path, src_dir.join("main.rs"));

    let cargo_status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(deploy_dir.join("Cargo.toml"))
        .arg("--release")
        .status();

    if let Ok(status) = cargo_status {
        if status.success() {
            let built = deploy_dir.join("target").join("release").join(if cfg!(windows) { "mltv_deploy.exe" } else { "mltv_deploy" });
            if built.exists() {
                if !keep {
                    let _ = fs::remove_file(&rs_path);
                }
                let _ = fs::copy(&built, binary_name);
                println!("Deployed: {}", binary_name);
                return Ok(());
            }
        }
    }

    // Fallback to rustc
    let output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(binary_name)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run rustc: {}", e))?;

    if !output.status.success() {
        if !keep {
            let _ = fs::remove_file(&rs_path);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("rustc error:\n{}", stderr);
        anyhow::bail!("rustc compilation failed");
    }

    if !keep {
        fs::remove_file(&rs_path)
            .map_err(|e| anyhow::anyhow!("failed to remove temp file: {}", e))?;
    }

    println!("Deployed: {}", binary_name);
    Ok(())
}

fn run_file(file: &str, args: &[String]) -> anyhow::Result<()> {
    let source_path = Path::new(file);
    if !source_path.exists() {
        anyhow::bail!("file '{}' not found", file);
    }

    let source = fs::read_to_string(source_path)
        .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", file, e))?;

    let tokens = tokenize(&source).map_err(|e| anyhow::anyhow!("tokenization error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| anyhow::anyhow!("parse error: {}", e))?;

    let source_dir = source_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let rust_code = transpile_with_dir(&program, &source_dir)
        .map_err(|e| anyhow::anyhow!("transpilation error: {}", e))?;

    let temp_dir = std::env::temp_dir().join("mltv_run");
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let stem = source_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let rs_path = src_dir.join("main.rs");
    let binary_path = temp_dir.join(format!("{}.bin", stem));

    let cargo_toml = r#"[package]
name = "mltv_program"
version = "0.1.0"
edition = "2021"

[dependencies]
serde_json = "1"
rand = "0.8"
"#;
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)
        .map_err(|e| anyhow::anyhow!("failed to write Cargo.toml: {}", e))?;

    fs::write(&rs_path, &rust_code)
        .map_err(|e| anyhow::anyhow!("failed to write '{}': {}", rs_path.display(), e))?;

    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(temp_dir.join("Cargo.toml"))
        .arg("--release")
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run cargo: {}", e))?;

    if !status.success() {
        anyhow::bail!("cargo build failed");
    }

    let built_binary = temp_dir.join("target").join("release").join(if cfg!(windows) { "mltv_program.exe" } else { "mltv_program" });
    if !built_binary.exists() {
        anyhow::bail!("cargo did not produce expected binary");
    }

    fs::copy(&built_binary, &binary_path)
        .map_err(|e| anyhow::anyhow!("failed to copy binary: {}", e))?;

    let mut cmd = Command::new(&binary_path);
    cmd.args(args);
    let exit_status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run binary: {}", e))?;

    if !exit_status.success() {
        anyhow::bail!("program exited with code {:?}", exit_status.code());
    }

    Ok(())
}
