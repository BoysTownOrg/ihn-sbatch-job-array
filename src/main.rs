use anyhow::{anyhow, Context};
use clap::Parser;
use std::{io::Write, process::ExitStatus};

#[derive(Parser)]
#[command(version = option_env!("IHN_SBATCH_JOB_ARRAY_VERSION").unwrap_or("debug"))]
struct Args {
    #[arg(long)]
    input: std::path::PathBuf,
    command: String,
    #[arg(long)]
    max_tasks: Option<String>,
    #[arg(long)]
    sbatch_path: Option<std::path::PathBuf>,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    sbatch_args: Vec<String>,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(status) => {
            if status.success() {
                std::process::ExitCode::SUCCESS
            } else {
                eprintln!("ERROR: Something went wrong...");
                std::process::ExitCode::FAILURE
            }
        }
        Err(what) => {
            eprintln!("ERROR: {what:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<ExitStatus> {
    let args = Args::parse();
    let input = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Unable to read input file, {:?}", args.input))?;
    let lines = input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let mut sbatch = std::process::Command::new(args.sbatch_path.unwrap_or("sbatch".into()))
        .arg(format!(
            "--array=0-{}%{}",
            lines.len() - 1,
            args.max_tasks.unwrap_or("16".to_string())
        ))
        .args(args.sbatch_args)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("Unable to invoke sbatch")?;
    if let Some(mut stdin) = sbatch.stdin.take() {
        writeln!(stdin, "#!/bin/bash")?;
        writeln!(stdin, "export TMPDIR=/ssd/home/$USER/TEMP")?;
        write!(stdin, "INPUT=(")?;
        for line in lines {
            write!(stdin, "\"{line}\" ")?;
        }
        writeln!(stdin, ")")?;
        writeln!(
            stdin,
            "srun --ntasks=1 \"{}\" \"${{INPUT[$SLURM_ARRAY_TASK_ID]}}\"",
            args.command
        )?;
    } else {
        return Err(anyhow!("Unable to take stdin of sbatch"));
    }
    Ok(sbatch.wait()?)
}
