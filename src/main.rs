pub mod file_info;
pub mod node;
pub mod project;

use std::{
    env,
    fmt::{Display, Formatter},
    fs::File,
    io::Write,
    path::Path,
    rc::Rc,
};

use itertools::Itertools;
use paste::paste;

use tstd::tenum::{tenum, TEnum, TEnumMap};

use crate::{node::Node, project::Project};

tenum!(ParamType {
    Project: |&'static| str = "p",
    EntryPoint: |&'static| str = "e",
    Output: |&'static| str = "o"
});

impl From<&str> for ParamType {
    fn from(val: &str) -> Self {
        match val {
            "p" => ParamType::Project,
            "e" => ParamType::EntryPoint,
            "o" => ParamType::Output,
            _ => panic!("Invalid Argument: {}", val),
        }
    }
}

impl Display for ParamType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ParamType::Project => "Project",
                ParamType::EntryPoint => "Entry Point",
                ParamType::Output => "Output File",
            }
        )
    }
}

const PARAM_SYMBOL: &'static str = "~";

fn param_exists_err(param_types: &[ParamType], map: &TEnumMap<ParamType, &str>) -> Option<String> {
    let mut res = String::new();

    for &param_type in param_types {
        let param = map.get(param_type);

        if param.unwrap_or(&"").is_empty() {
            res.push_str(&format!(
                "{}{}",
                if res.is_empty() { "" } else { " & " },
                param_type
            ));
        }
    }

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

/// ~p={project_path} (has to be absolute path)
/// ~e={entry_point} (has to be absolute path)
/// ~o={output_file} (has to be absolute path)
fn main() {
    let args: Vec<String> = env::args().collect();

    let mut params = TEnumMap::empty();

    for arg in args.iter() {
        if arg.contains(PARAM_SYMBOL) {
            params.insert(ParamType::from(&arg[1..2]), &arg[3..]);
        }
    }

    if let Some(err) = param_exists_err(
        &[ParamType::Project, ParamType::EntryPoint, ParamType::Output],
        &params,
    ) {
        let multiple = err.contains("&");
        panic!(
            "{} File Path{} {} not set!",
            err,
            if multiple { "s" } else { "" },
            if multiple { "were" } else { "was" }
        )
    }

    let mut project = Project::create(params.get(ParamType::Project).unwrap().to_owned());
    let entry_point_file_info =
        Rc::new(project.create_file_info(params.get(ParamType::EntryPoint).unwrap().to_owned()));

    let root_node = Node::create(&entry_point_file_info, None);

    let recursive_paths = Node::traverse(&root_node, &mut project);

    let mut file = File::create(Path::new(params.get(ParamType::Output).unwrap()))
        .expect("Couldn't create the output file!");

    for (file_name, paths) in recursive_paths.iter() {
        file.write_all(b"------------------------------------------------\n")
            .expect("Couldn't write to the output file");

        file.write_all((format!("{}:\n", file_name)).as_bytes())
            .expect("Couldn't write to the output file");

        let output_paths: Vec<&Vec<String>> = paths
            .iter()
            .sorted_by(|path1, path2| Ord::cmp(&path1.len(), &path2.len()))
            .collect();

        for path in output_paths {
            file.write_all(format!("\t{}\n", path.join("->")).as_bytes())
                .expect("Couldn't write to the output file");
        }

        file.write_all("------------------------------------------------\n".as_bytes())
            .expect("Couldn't write to the output file");
    }
}
