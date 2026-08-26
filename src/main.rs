//! TraceSeed turns a failing run into an editable reproduction capsule.
//!
//! `capture` runs a command until it fails, then writes down the exact conditions
//! that produced the failure. `replay` puts those conditions back and reports
//! whether the failure came back with them.

use std::collections::BTreeMap;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

const ATTEMPTS: u32 = 200;
const TRACE_FILE: &str = "failure.trace";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["capture", command] => capture(command, ""),
        ["capture", command, input] => capture(command, input),
        ["replay", file] => replay(file),
        _ => {
            eprintln!("usage:\n  trace-seed capture <command> [input]\n  trace-seed replay <file.trace>");
            ExitCode::FAILURE
        }
    }
}

/// Everything needed to put the program back in the state that failed.
struct Trace {
    command: String,
    seed: u64,
    input: String,
    env: BTreeMap<String, String>,
    exit: i32,
}

impl Trace {
    fn parse(text: &str) -> Trace {
        let mut trace = Trace {
            command: String::new(),
            seed: 0,
            input: String::new(),
            env: BTreeMap::new(),
            exit: 1,
        };
        for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
            match key.trim() {
                "command" => trace.command = value.into(),
                "seed" => trace.seed = value.trim().parse().unwrap_or(0),
                "input" => trace.input = value.into(),
                "exit" => trace.exit = value.trim().parse().unwrap_or(1),
                other => {
                    if let Some(name) = other.strip_prefix("env.") {
                        trace.env.insert(name.into(), value.into());
                    }
                }
            }
        }
        trace
    }

    /// One field per line, sorted, so a trace diffs cleanly in Git.
    fn render(&self) -> String {
        let mut text = format!(
            "command={}\nseed={}\ninput={}\n",
            self.command, self.seed, self.input
        );
        for (name, value) in &self.env {
            text += &format!("env.{name}={value}\n");
        }
        text + &format!("exit={}\n", self.exit)
    }

    /// Runs the command under exactly the conditions this trace describes.
    fn run(&self) -> (i32, String) {
        let result = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .env("SEED", self.seed.to_string())
            .env("INPUT", &self.input)
            .envs(&self.env)
            .output()
            .expect("could not start `sh`");
        let mut output = String::from_utf8_lossy(&result.stdout).into_owned();
        output += &String::from_utf8_lossy(&result.stderr);
        (result.status.code().unwrap_or(-1), output)
    }
}

/// Runs the command with a fresh seed until it fails, then writes that run down.
fn capture(command: &str, input: &str) -> ExitCode {
    let watched = env::var("TRACE_ENV").unwrap_or_default();
    let env: BTreeMap<String, String> = watched
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| (name.to_string(), env::var(name).unwrap_or_default()))
        .collect();

    for attempt in 0..ATTEMPTS {
        let mut trace = Trace {
            command: command.into(),
            seed: seed(attempt),
            input: input.into(),
            env: env.clone(),
            exit: 0,
        };
        let (exit, output) = trace.run();
        if exit != 0 {
            trace.exit = exit;
            fs::write(TRACE_FILE, trace.render()).expect("could not write the trace");
            print!("{output}");
            println!("\ncaptured after {} runs -> {TRACE_FILE}", attempt + 1);
            return ExitCode::SUCCESS;
        }
    }
    println!("no failure in {ATTEMPTS} runs");
    ExitCode::SUCCESS
}

/// Puts the recorded conditions back and reports whether the failure returned.
fn replay(file: &str) -> ExitCode {
    let trace = Trace::parse(&fs::read_to_string(file).expect("could not read the trace"));
    let (exit, output) = trace.run();
    print!("{output}");
    if exit == trace.exit {
        println!("REPRODUCED (exit {exit}, as recorded)");
        ExitCode::SUCCESS
    } else {
        println!("NOT REPRODUCED (exit {exit}, the trace records exit {})", trace.exit);
        ExitCode::FAILURE
    }
}

/// A short, readable, unpredictable seed.
fn seed(attempt: u32) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before 1970")
        .as_nanos() as u64;
    (now ^ u64::from(attempt).wrapping_mul(0x9E37_79B9_7F4A_7C15)) % 100_000
}
