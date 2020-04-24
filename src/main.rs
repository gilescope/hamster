#[macro_use]
extern crate serde_derive;
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use subprocess::Exec;
use yaml_merge_keys::merge_keys_serde;

type DynErr = Box<dyn std::error::Error + 'static>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Job {
    stage: Option<String>,
    before_script: Option<Vec<String>>,
    script: Option<Vec<String>>,
    variables: Option<HashMap<String, String>>,
    extends: Option<String>,
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

fn run(gitlab_file: &Path, job: &str) -> Result<(), Box<dyn std::error::Error>> {
    let f = std::fs::File::open(&gitlab_file)?;
    let raw_yaml = serde_yaml::from_reader(f)?;

    let val: serde_yaml::Value = merge_keys_serde(raw_yaml).unwrap();
    if let serde_yaml::Value::Mapping(map) = val {
        let mut global_vars: HashMap<String, String> = HashMap::new();
        for (k, v) in map.iter() {
            if let Value::String(key) = k {
                if key == "variables" {
                    let global_var_map: Mapping = serde_yaml::from_value(v.clone())?;
                    for (key, value) in global_var_map {
                        if let Value::String(key) = key {
                            if let Value::String(value) = value {
                                global_vars.insert(key, value);
                            }
                        }
                    }
                    // Globally defined variables. These should be ignored if inherit:varialbes:false
                }
            }
        }
        for (k, v) in map.iter() {
            if let Value::String(key) = k {
                if key != "stages" {
                    //Found target.
                    let j: Result<Job, _> = serde_yaml::from_value(v.clone());
                    if let Ok(j) = j {
                        if *key == job || j.stage.is_some() && (job == *j.stage.as_ref().unwrap()) {
                            run_job(&global_vars, &map, key, j);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn set_vars(map: &Mapping, name: &str, mut vars: &mut HashMap<String, String>) {
    let me: Job =
        serde_yaml::from_value(map.get(&Value::String(name.to_string())).unwrap().clone()).unwrap();
    if let Some(parent) = me.extends {
        set_vars(map, &parent, &mut vars);
    }
    if let Some(me_vars) = me.variables {
        for (key, value) in me_vars {
            vars.insert(key, value);
        }
    }
}

fn run_job(global_vars: &HashMap<String, String>, map: &Mapping, key: &String, j: Job) {
    let mut local_vars = global_vars.clone();

    set_vars(&map, &key, &mut local_vars);

    if let Some(vars) = j.variables {
        local_vars.extend(vars);
    }

    if let Some(script) = j.before_script {
        run_script(script, &local_vars);
    }

    if let Some(script) = j.script {
        run_script(script, &local_vars);
    }
}

fn run_script(script: Vec<String>, local_vars: &HashMap<String, String>) {
    for line in script {
        let mut proc = Exec::shell(OsString::from(line));
        for (key, value) in local_vars.iter() {
            proc = proc.env(key, value);
        }
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
