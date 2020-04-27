use gitlab_ci_parser::*;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use subprocess::Exec;
type DynErr = Box<dyn std::error::Error + 'static>;

struct Config {
    verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { verbose: false }
    }
}

fn main() -> Result<(), DynErr> {
    let mut dir: &Path = &env::current_dir()?;
    while !Path::join(dir, Path::new(".gitlab-ci.yml")).exists() {
        dir = dir.parent().unwrap();
    }

    let args: Vec<String> = std::env::args().collect();

    let target = if args.len() < 2 {
        None
    } else {
        Some(args[1].to_owned())
    };
    let mut config = Config::default();
    for arg in &args {
        if arg == "--verbose" || arg == "-v" {
            config.verbose = true;
        }
    }
    run(
        &Path::join(&dir, Path::new(".gitlab-ci.yml")),
        target,
        &config,
    )
}

fn run(
    gitlab_file: &Path,
    job: Option<String>,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let gitlab_config = gitlab_ci_parser::parse(gitlab_file)?;

    if let Some(job) = job {
        for (key, j) in gitlab_config.jobs.iter() {
            if key == &job || j.stage.is_some() && (&job == j.stage.as_ref().unwrap()) {
                run_job(&gitlab_config, j, config);
            }
        }
    } else {
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

fn set_vars(job: &Job, mut vars: &mut HashMap<String, String>) {
    if let Some(ref parent) = job.extends_job {
        set_vars(&parent, &mut vars);
    }
    if let Some(ref me_vars) = job.variables {
        for (key, value) in me_vars {
            vars.insert(key.clone(), value.clone());
        }
    }
}

fn run_job(gitlab_config: &GitlabCIConfig, j: &Job, config: &Config) {
    let mut local_vars = gitlab_config.variables.clone();

    set_vars(&j, &mut local_vars);

    if let Some(ref vars) = j.variables {
        local_vars.extend(vars.clone());
    }

    if let Some(ref script) = j.before_script {
        run_script(script, &local_vars, config);
    }

    if let Some(ref script) = j.script {
        run_script(script, &local_vars, config);
    }
}

fn run_script(script: &Vec<String>, local_vars: &HashMap<String, String>, config: &Config) {
    for line in script {
        let mut proc = Exec::shell(OsString::from(line));
        for (key, value) in local_vars.iter() {
            proc = proc.env(key, value);
            if config.verbose {
                println!("Env: {}={}", key, value)
            };
        }
        if config.verbose {
            println!("Cmd: {}", line)
        };
        proc.join().unwrap();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn hello() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("print_hello".into()), &Config::default())
    }

    #[test]
    pub fn goodbye_with_variable() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("print_goodbye".into()), &Config::default())
    }

    #[test]
    pub fn all_in_stage_run() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, Some("primary_stage".into()), &Config::default())
    }
}
