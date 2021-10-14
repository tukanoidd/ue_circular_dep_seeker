pub mod project;
pub mod file_info;
pub mod node;

use std::{
    env,
    rc::Rc,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use itertools::Itertools;

use substring::Substring;

use crate::{
    node::Node,
    project::Project
};

const PARAM_SYMBOL: &'static str = "~";

/// ~p={project_path} (has to be absolute path)
/// ~e={entry_point} (has to be absolute path)
/// ~o={output_file} (has to be absolute path)
fn main() {
    let args: Vec<String> = env::args().collect();

    let mut project_path = "";
    let mut entry_point = "";
    let mut output_file = "";

    for arg in args.iter() {
        if arg.contains(PARAM_SYMBOL) {
            let arg_len = arg.len();
            let arg_val = arg.substring(3, arg_len);

            if arg.starts_with(format!("{}p", PARAM_SYMBOL).as_str()) {
                project_path = arg_val.clone();
            } else if arg.starts_with(format!("{}e", PARAM_SYMBOL).as_str()) {
                entry_point = arg_val.clone();
            } else if arg.starts_with(format!("{}o", PARAM_SYMBOL).as_str()) {
                output_file = arg_val.clone();
            }
        }
    }

    if project_path.is_empty() || entry_point.is_empty() || output_file.is_empty() {
        panic!("Project/Entry Point/Output File Path was not set!")
    }

    let mut project = Project::create(project_path.to_owned());
    let entry_point_file_info = Rc::new(
        project.create_file_info(entry_point.to_owned())
    );

    let root_node = Node::create(
        &entry_point_file_info,
        None,
    );

    let recursive_paths =
        Node::traverse(&root_node, &mut project);

    let mut file = File::create(Path::new(output_file))
        .expect("Couldn't create the output file!");

    for (file_name, paths) in recursive_paths.iter() {
        file.write("------------------------------------------------\n".as_bytes())
            .expect("Couldn't write to the output file");

        file.write((format!("{}:\n", file_name)).as_bytes())
            .expect("Couldn't write to the output file");

        let output_paths: Vec<&Vec<String>> = paths.iter()
            .sorted_by(|path1, path2| {
                Ord::cmp(&path1.len(), &path2.len())
            })
            .collect();

        for path in output_paths {
            file.write(format!("\t{}\n", path.join("->")).as_bytes())
                .expect("Couldn't write to the output file");
        }

        file.write("------------------------------------------------\n".as_bytes())
            .expect("Couldn't write to the output file");
    }
}
