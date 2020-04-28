use gitlab_ci_parser::*;
use shellexpand;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use subprocess::Exec;
use tracing::{debug, info, Level};
use tracing_subscriber;

type DynErr = Box<dyn std::error::Error + 'static>;
type Vars = BTreeMap<String, String>;

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
        if args[1] == "--version" {
            println!("hamster v{}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        if args[1] == "--debug" {
            let gitlab_file = &Path::join(&dir, Path::new(".gitlab-ci.yml"));
            let gitlab_config = gitlab_ci_parser::parse(gitlab_file)?;
            println!("{:#?}", gitlab_config);
            return Ok(());
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
    debug!("paring {:?}", &gitlab_file);
    let gitlab_config = gitlab_ci_parser::parse(gitlab_file)?;
    debug!("paring {:?} finished", &gitlab_file);

    if let Some(job_name) = job {
        debug!("finding {}", &job_name);
        if let Some(job) = gitlab_config.lookup_job(&job_name) {
            debug!("found {}", &job_name);
            run_job(&gitlab_config, &job);
        } else {
            info!("Can't find job {}", job_name);
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

fn run_job(gitlab_config: &GitlabCIConfig, job: &Job) {
    let mut vars: Vars = Vars::new();
    let build_dir = gitlab_config
        .file
        .parent()
        .expect("gitlab config not in a dir!")
        .to_str()
        .expect("odd path");

    vars.insert("CI_BUILDS_DIR".into(), build_dir.to_owned());
    vars.insert("CI_PROJECT_DIR".into(), build_dir.to_owned());

    vars.insert(
        "CI_CONFIG_PATH".into(),
        gitlab_config.file.to_str().expect("odd path").to_owned(),
    );

    let info = git_info::get();
    vars.insert(
        "CI_COMMIT_SHA".into(),
        "XXXXdirtyXXXXdirtyXXXXdirtyXXXXdirtyXXXX".into(),
    );
    vars.insert("CI_COMMIT_SHORT_SHA".into(), "DirtySHA".into()); // 8 chars for gitlab short
    vars.insert(
        "CI_COMMIT_BRANCH".into(),
        info.current_branch
            .as_ref()
            .unwrap_or(&"Unknown".into())
            .clone(),
    );
    vars.insert(
        "CI_COMMIT_REF_NAME".into(),
        info.current_branch.unwrap_or("Unknown".to_string()).clone(),
    );
    vars.insert("CI_COMMIT_TITLE".into(), "Working Copy".into());

    vars.extend(gitlab_config.get_merged_variables());
    vars.extend(job.get_merged_variables());

    if let Some(ref script) = job.before_script {
        run_script(script, &vars);
    }

    if let Some(ref script) = job.script {
        run_script(script, &vars);
    }
}

fn run_script(script: &Vec<String>, local_vars: &Vars) {
    for line in script {
        let mut proc = Exec::shell(OsString::from(line));
        for (key, value) in local_vars.iter() {
            let value = expand_vars(value, local_vars);
            proc = proc.env(key, &value);
            debug!("Env: {}={}", key, value);
        }
        info!("Cmd: {}", line);
        proc.join().expect("Process returned non-zero exit code");
    }
}

/// The website says to use go's os.expand function's semantics:
fn expand_vars(var: &str, vars: &Vars) -> String {
    shellexpand::env_with_context_no_errors(var, |key: &str| {
        if let Some(value) = vars.get(key) {
            return Some(value.to_owned());
        }
        if let Ok(value) = env::var(key) {
            return Some(value);
        }
        return None;
    })
    .to_string()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::path::PathBuf;

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
