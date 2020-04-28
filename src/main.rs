use gitlab_ci_parser::*;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use subprocess::Exec;
use tracing::{debug, info, Level};
use tracing_subscriber;

type DynErr = Box<dyn std::error::Error + 'static>;

fn main() -> Result<(), DynErr> {
    let mut dir: &Path = &env::current_dir()?;
    while !Path::join(dir, Path::new(".gitlab-ci.yml")).exists() {
        dir = dir
            .parent()
            .expect("Can't find .gitlab-ci.yml in a parent dir!");
    }

    let args: Vec<String> = std::env::args().collect();

    let target = if args.len() < 2 {
        None
    } else {
        if args[1] == "--version" || args[1] == "-v" {
            println!("hamster v{}", env!("CARGO_PKG_VERSION"));
        }
        Some(args[1].to_owned())
    };

    let mut lev = Level::INFO;
    for arg in &args {
        if arg == "--verbose" || arg == "-v" {
            lev = Level::TRACE;
        }
    }

    //Enable ansi support on win10
    #[cfg(windows)]
    let _enabled = ansi_term::enable_ansi_support();

    let subscriber = tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(lev)
        // builds the subscriber.
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        run(&Path::join(&dir, Path::new(".gitlab-ci.yml")), target)
    })
}

fn run(gitlab_file: &Path, job: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let gitlab_config = gitlab_ci_parser::parse(gitlab_file)?;

    if let Some(job) = job {
        for (key, j) in gitlab_config.jobs.iter() {
            if key == &job || j.stage.is_some() && (&job == j.stage.as_ref().unwrap()) {
                run_job(&gitlab_config, j);
            }
        }
    } else {
        println!("Global variables:");
        for (k, v) in gitlab_config.get_merged_variables() {
            println!("\t{}={}", k, v);
        }
        println!();
        println!("Found targets:");
        let mut results = vec![];
        print_config(&gitlab_config, &mut results);
        for r in results {
            println!("\t{}", r);
        }
    }

    Ok(())
}

fn print_config(config: &GitlabCIConfig, results: &mut Vec<String>) {
    for (job, _) in &config.jobs {
        if !results.contains(&job) {
            results.push(job.clone());
        }
    }
    if let Some(ref parent) = config.parent {
        print_config(&parent, results)
    }
}

fn run_job(gitlab_config: &GitlabCIConfig, j: &Job) {
    let mut vars = gitlab_config.get_merged_variables();
    vars.extend(j.get_merged_variables());

    if let Some(ref script) = j.before_script {
        run_script(script, &vars);
    }

    if let Some(ref script) = j.script {
        run_script(script, &vars);
    }
}

fn run_script(script: &Vec<String>, local_vars: &HashMap<String, String>) {
    for line in script {
        let mut proc = Exec::shell(OsString::from(line));
        for (key, value) in local_vars.iter() {
            proc = proc.env(key, value);
            debug!("Env: {}={}", key, value);
        }
        info!("Cmd: {}", line);
        proc.join().expect("Process returned non-zero exit code");
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::path::PathBuf;
    //use subprocess::Exec;

    #[test]
    pub fn hello() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("print_hello".into()))
    }

    #[test]
    pub fn goodbye_with_variable() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("print_goodbye".into()))
    }

    #[test]
    pub fn all_in_stage_run() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("primary_stage".into()))
    }

    // #[test]
    // pub fn all_in_stage_run() -> Result<(), DynErr> {
    //     let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    //     let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
    //     run(p, Some("primary_stage".into()))
    // }

    // #[test]
    // pub fn no_args() {
    //     let exe: &'static str = env!("CARGO_BIN_EXE_HAMSTER");
    //     assert!(Exec::cmd(exe).join().is_ok());
    // }
}
