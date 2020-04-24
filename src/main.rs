#[macro_use]
extern crate serde_derive;
use serde_yaml::Value;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use subprocess::Exec;

type DynErr = Box<dyn std::error::Error + 'static>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Target {
    stage: Option<String>,
    before_script: Option<Vec<String>>,
    script: Option<Vec<String>>,
    variables: Option<HashMap<String, String>>,
}

fn main() -> Result<(), DynErr> {
    let mut dir: &Path = &env::current_dir()?;
    while !Path::join(dir, Path::new(".gitlab-ci.yml")).exists() {
        dir = dir.parent().unwrap();
    }
    let args: Vec<String> = std::env::args().collect();
    let target = &args[1];
    run(&Path::join(&dir, Path::new(".gitlab-ci.yml")), target)
}

fn run(gitlab_file: &Path, target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let f = std::fs::File::open(&gitlab_file)?;
    let map: serde_yaml::Mapping = serde_yaml::from_reader(f)?;

    for (k, v) in map.iter() {
        if let Value::String(key) = k {
            if key != "stages" {
                //Found target.
                let t: Result<Target, _> = serde_yaml::from_value(v.clone());
                if let Ok(t) = t {
                    if *key == target || t.stage.is_some() && (target == *t.stage.as_ref().unwrap())
                    {
                        if let Some(script) = t.before_script {
                            for line in script {
                                let mut proc = Exec::shell(OsString::from(line));
                                if let Some(ref vars) = t.variables {
                                    for (key, value) in vars.iter() {
                                        proc = proc.env(key, value);
                                    }
                                }
                                proc.join().unwrap();
                            }
                        }
                        if let Some(script) = t.script {
                            for line in script {
                                let mut proc = Exec::shell(OsString::from(line));
                                if let Some(ref vars) = t.variables {
                                    for (key, value) in vars.iter() {
                                        proc = proc.env(key, value);
                                    }
                                }
                                proc.join().unwrap();
                            }
                        }
                    } else {
                        println!("skipping {:?} {:?}", k, t);
                    }
                } else {
                    println!("skipping {:?} {:?}", k, t);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn hello() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, "print_hello")
    }

    #[test]
    pub fn goodbye_with_variable() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, "print_goodbye")
    }

    #[test]
    pub fn all_in_stage_run() -> Result<(), DynErr> {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let p = &PathBuf::from(Path::join(&root, "examples/simple/.gitlab-ci.yml"));
        run(p, "primary_stage")
    }
}
