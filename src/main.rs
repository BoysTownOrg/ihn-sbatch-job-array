use anyhow::{anyhow, Context};
use clap::Parser;
use std::{io::Write, process::ExitStatus};

#[derive(Parser)]
#[command(version = option_env!("IHN_SBATCH_JOB_ARRAY").unwrap_or("debug"))]
struct Args {
    #[arg(long)]
    input: std::path::PathBuf,
    command: String,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(status) => {
            if let Some(code) = status.code() {
                std::process::ExitCode::from(code.try_into().unwrap_or(0))
            } else {
                std::process::ExitCode::SUCCESS
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
    let mut sbatch = std::process::Command::new("sbatch")
        .arg(format!("--array=0-{}", lines.len() - 1))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("Unable to invoke sbatch")?;
    if let Some(mut stdin) = sbatch.stdin.take() {
        writeln!(stdin, "#!/bin/bash")?;
        writeln!(stdin, "export TMPDIR=/home/ssd/$USER/TEMP")?;
        write!(stdin, "INPUT=(")?;
        for line in lines {
            write!(stdin, "\"{line}\" ")?;
        }
        writeln!(stdin, ")")?;
        writeln!(
            stdin,
            "srun \"{}\" \"INPUT[$SLURM_ARRAY_TASK_ID]\"",
            args.command
        )?;
    } else {
        return Err(anyhow!("Unable to take stdin of sbatch"));
    }
    Ok(sbatch.wait()?)
}
