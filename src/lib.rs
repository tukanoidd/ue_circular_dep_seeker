#[macro_use]
extern crate log;

pub mod file_info {
    use std::{
        cell::RefCell,
        fmt::{Debug, Display, Formatter},
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        rc::Rc,
    };

    use anyhow::*;

    #[derive(Eq, PartialEq, Hash)]
    pub enum FileType {
        Header,
        Source,
        Inline,
    }

    impl Display for FileType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                FileType::Header => write!(f, "Header"),
                FileType::Source => write!(f, "Source"),
                FileType::Inline => write!(f, "Inline"),
            }
        }
    }

    #[derive(Eq, PartialEq, Hash)]
    pub struct FileInfo {
        pub abs_path: String,
        pub file_name: String,
        pub module: String,
        pub file_type: FileType,
        pub includes: Vec<String>,
        pub processed: bool,
    }

    impl FileInfo {
        pub fn create(
            abs_path: &str,
            modules: &[(String, Vec<String>)],
        ) -> Result<Rc<RefCell<FileInfo>>> {
            let file = File::open(Path::new(abs_path))?;

            let file_name = abs_path.split('/').last().unwrap();
            let file_type_str = file_name.split('.').last().unwrap();

            let file_type = match file_type_str {
                "h" | "hpp" => FileType::Header,
                "c" | "cpp" => FileType::Source,
                "inl" => FileType::Inline,
                _ => bail!(
                    "{}",
                    format!("File type is not supported: '{}'", file_type_str)
                ),
            };

            let file_lines = BufReader::new(file).lines();

            let mut includes = vec![];

            for mut line in file_lines.flatten() {
                if line.contains("#include") {
                    if line.contains(".generated.") || line.contains(".gen.") {
                        continue;
                    }

                    line = line.trim().to_owned();

                    let l_split = line.split(' ');

                    includes.push(l_split.last().unwrap().replace('\"', "").to_owned());
                }
            }

            let module = modules
                .iter()
                .rfind(|(modl, _include_paths)| abs_path.contains(modl.as_str()));

            let module = match module {
                Some(module) => module.0.clone(),
                None => bail!("Couldn't find the module of the file: {}", abs_path),
            };

            Ok(Rc::new(RefCell::new(Self {
                abs_path: abs_path.to_string(),
                file_name: file_name.to_owned(),
                module,
                file_type,
                includes,
                processed: false,
            })))
        }
    }

    impl Debug for FileInfo {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "FileInfo(")?;
            writeln!(f, "\tAbsolute Path: {}", self.abs_path)?;
            writeln!(f, "\tFile Name: {}", self.file_type)?;
            writeln!(f, "\tModule: {}", self.module)?;
            writeln!(f, "\tFile Type: {}", self.file_type)?;
            writeln!(f, "\tIncludes: {:?}", self.includes)?;
            writeln!(f, "\tProcessed: {}", self.processed)?;
            writeln!(f, ")")
        }
    }
}

pub mod node {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        fmt::{Debug, Formatter},
        rc::Rc,
    };

    use itertools::Itertools;

    use crate::{file_info::FileInfo, project::Project};

    #[derive(Eq)]
    pub struct Node {
        file_info: Rc<RefCell<FileInfo>>,
        prev: Option<Rc<RefCell<Node>>>,
        children: Vec<Rc<RefCell<Node>>>,
        node_path: Vec<Rc<RefCell<FileInfo>>>,
    }

    impl Node {
        pub fn create(
            file_info: &Rc<RefCell<FileInfo>>,
            prev: Option<Rc<RefCell<Node>>>,
        ) -> Rc<RefCell<Self>> {
            let mut node_path = vec![];

            if let Some(previous) = prev.clone() {
                node_path.extend((*previous).borrow().node_path.clone());
            }
            node_path.push(file_info.clone());

            Rc::new(RefCell::new(Self {
                file_info: file_info.clone(),
                prev,
                children: vec![],
                node_path,
            }))
        }

        pub fn traverse(
            starting_node: &Rc<RefCell<Node>>,
            project: &mut Project,
        ) -> HashMap<String, HashSet<Vec<String>>> {
            let mut recursive_paths: HashMap<String, HashSet<Vec<String>>> = HashMap::new();

            let mut current = starting_node.clone();

            loop {
                let current_processed = (*(*current).borrow().file_info).borrow().processed;

                // If the current node is already processed
                if current_processed {
                    let current_prev = (*current).borrow().prev.clone();

                    // Go Back
                    if let Some(previous) = current_prev {
                        current = previous;
                        continue;
                    } else {
                        break;
                    }
                }

                // If it's not yet fully processed
                // Check if the node has children
                let mut current_children = (*current).borrow().children.clone();
                if current_children.is_empty() {
                    // If the doesn't have children yet
                    // Check if the file o the node actually has any includes
                    let current_file_info = (*current).borrow().file_info.clone();
                    if !(*current_file_info).borrow().includes.is_empty() {
                        // If there are any includes, create node children
                        Self::create_node_children(current.clone(), project);
                    } else {
                        // If there was non in the first place, we can count this node as a processed
                        // one and skip loop iteration
                        (*(*current).borrow_mut().file_info).borrow_mut().processed = true;
                        continue;
                    }
                }

                // If the node has children, lets fine an unprocessed one
                current_children = (*current).borrow().children.clone();
                if let Some(unprocessed_child) = current_children
                    .iter()
                    .find(|&child| !(*(*child.clone()).borrow().file_info).borrow().processed)
                {
                    // If we find one, we check if it's not a recursive one
                    let (is_unprocessed_child_recursive, file_name) =
                        (*unprocessed_child.clone()).borrow().is_recursive();
                    if is_unprocessed_child_recursive {
                        // If it is recursive, it can be considered processed right away and we print
                        // out its path
                        (*(*unprocessed_child.clone()).borrow_mut().file_info)
                            .borrow_mut()
                            .processed = true;

                        let readable_path = (*unprocessed_child.clone()).borrow().readable_path();

                        let key = file_name.unwrap();

                        if let Some(path) = recursive_paths.get_mut(key.as_str()) {
                            path.insert(readable_path.clone());
                        } else {
                            let mut set = HashSet::new();
                            set.insert(readable_path.clone());

                            recursive_paths.insert(key, set);
                        }

                        info!("RECURSIVE PATH FOUND: {:?}", readable_path);
                    } else {
                        // If it isn't, we can go deeper into the tree
                        current = unprocessed_child.clone();
                    }
                } else {
                    // If there's none left, we can call this node processed and skip the loop iteration
                    (*(*current).borrow_mut().file_info).borrow_mut().processed = true;
                }
            }

            recursive_paths
        }

        fn create_node_children(node: Rc<RefCell<Node>>, project: &mut Project) {
            let file_info = node.borrow().file_info.clone();

            let node_children = (*file_info)
                .borrow()
                .includes
                .iter()
                .filter_map(|include| {
                    match project.get_file(include, &(*file_info).borrow().module) {
                        Ok(include_file_info) => {
                            Some(Node::create(&include_file_info, Some(node.clone())))
                        }
                        Err(_) => None,
                    }
                })
                .collect();

            node.borrow_mut().children = node_children;
        }

        fn is_recursive(&self) -> (bool, Option<String>) {
            let mut abs_paths = self
                .node_path
                .iter()
                .map(|file_info| (*file_info).borrow().abs_path.clone());

            if !abs_paths.all_unique() {
                (
                    true,
                    Some((*self.node_path.last().unwrap()).borrow().file_name.clone()),
                )
            } else {
                (false, None)
            }
        }

        fn readable_path(&self) -> Vec<String> {
            self.node_path
                .iter()
                .map(|node| (*node).borrow().file_name.clone())
                .collect()
        }
    }

    impl PartialEq for Node {
        fn eq(&self, other: &Self) -> bool {
            self.file_info == other.file_info && self.prev == other.prev
        }
    }

    impl Debug for Node {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "Node (")?;
            writeln!(f, "\tFile Info: {}", (*self.file_info).borrow().abs_path)?;
            writeln!(
                f,
                "\tPrevious Node: {}",
                match self.prev.clone() {
                    Some(previous_node) => (*(*previous_node).borrow().file_info)
                        .borrow()
                        .file_name
                        .clone(),
                    None => "None".to_owned(),
                }
            )?;
            writeln!(
                f,
                "\tChildren: {:?}",
                self.children
                    .iter()
                    .map(|child| { (*(**child).borrow().file_info).borrow().file_name.clone() })
                    .collect::<Vec<String>>()
            )?;
            writeln!(f, "\tNode Path: {:?}", self.node_path)?;
            writeln!(f, ")")
        }
    }
}

pub mod project {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        fmt::{Debug, Formatter},
        fs::File,
        io::{BufRead, BufReader},
        iter::FromIterator,
        path::Path,
        rc::Rc,
    };

    use anyhow::*;

    use crate::file_info::FileInfo;

    pub struct Project {
        pub root_path: String,
        pub modules: Vec<(String, Vec<String>)>,
        pub files: Vec<Rc<RefCell<FileInfo>>>,
        pub circular_dependency_paths: HashSet<Vec<String>>,
    }

    impl Project {
        pub fn create(project_path: &str) -> Result<Self> {
            let cmake_lists_file = File::open(Path::new(
                (project_path.to_string() + "/CMakeLists.txt").as_str(),
            ))?;

            let mut modules: HashMap<String, HashSet<String>> = HashMap::new();

            let cmake_lists_lines = BufReader::new(cmake_lists_file).lines();

            for cmake_lists_line in cmake_lists_lines.flatten() {
                let stripped_cll = cmake_lists_line.replace(' ', "");

                if stripped_cll.contains("include(") {
                    let include = stripped_cll.replace("include(\"", "").replace("\")", "");

                    if !include.contains("includes") {
                        continue;
                    }

                    let include_cmake_file = File::open(Path::new(include.clone().as_str()))?;

                    let include_cmake_file_lines = BufReader::new(include_cmake_file).lines();

                    for include_cmake_file_line in include_cmake_file_lines.flatten() {
                        let stripped_ifl = include_cmake_file_line.replace(' ', "");

                        if stripped_ifl.contains('\"') {
                            let inc_folder = stripped_ifl
                                .replace('\"', "")
                                .replace('\t', "")
                                .replace('\n', "");

                            if inc_folder.contains("Intermediate") {
                                continue;
                            }

                            let start_ind = match inc_folder.rfind("Engine/") {
                                Some(start_ind) => start_ind,
                                None => bail!("Couldn't get start_ind"),
                            };

                            let module = inc_folder[start_ind..]
                                .replace("/Public", "")
                                .replace("/Private", "");

                            if modules.contains_key(module.clone().as_str()) {
                                modules
                                    .get_mut(module.clone().as_str())
                                    .unwrap()
                                    .insert(inc_folder);
                            } else {
                                modules.insert(module.clone(), HashSet::from_iter([inc_folder]));
                            }
                        }
                    }
                }
            }

            let mut res_modules: Vec<(String, Vec<String>)> = modules
                .iter()
                .map(|(module, include_paths)| {
                    (
                        module.clone(),
                        include_paths.iter().cloned().collect::<Vec<String>>(),
                    )
                })
                .collect();
            res_modules.sort_by(|(mod1, _inc1), (mod2, _inc2)| Ord::cmp(&mod1.len(), &mod2.len()));

            Ok(Self {
                root_path: project_path.to_string(),
                modules: res_modules,
                files: vec![],
                circular_dependency_paths: HashSet::new(),
            })
        }

        pub fn create_file_info(&mut self, abs_path: &str) -> Result<Rc<RefCell<FileInfo>>> {
            let file_info = FileInfo::create(abs_path, &self.modules)?;

            self.files.push(file_info.clone());

            Ok(file_info)
        }

        pub fn get_file(
            &mut self,
            partial_path: &str,
            entry_module: &str,
        ) -> Result<Rc<RefCell<FileInfo>>> {
            // Check if root module actually exists
            let mut root_module = None;

            for modl in self.modules.clone() {
                if modl.0 == entry_module {
                    root_module = Some(modl);
                    break;
                }
            }

            // If it does
            if root_module.is_some() {
                let modl = root_module.clone().unwrap();

                if let std::result::Result::Ok(file) = self.get_file_in_module(modl, partial_path) {
                    return Ok(file);
                }
            }

            let other_modules: Vec<(String, Vec<String>)> = if let Some(root_mod) = root_module {
                self.modules
                    .iter()
                    .filter(|(modl, _include_paths)| modl != &root_mod.0)
                    .cloned()
                    .collect()
            } else {
                self.modules.clone()
            };

            for module in other_modules {
                if let std::result::Result::Ok(file) = self.get_file_in_module(module, partial_path)
                {
                    return Ok(file);
                }
            }

            bail!("Couldn't get the file");
        }

        fn get_file_in_module(
            &mut self,
            modl: (String, Vec<String>),
            partial_path: &str,
        ) -> Result<Rc<RefCell<FileInfo>>> {
            // Check if any of the paths inside of the module are viable for the file we're looking
            // for
            for include_path in modl.1.iter() {
                // Concatenating the include path and partial path
                let path_to_file = format!("{}/{}", include_path, partial_path);

                // If path exists on the computer
                if Path::new(path_to_file.as_str()).exists() {
                    // Return cached file info if it exists
                    return if let Some(file) = self
                        .files
                        .iter()
                        .find(|f| (*f).borrow().abs_path == path_to_file)
                    {
                        Ok(file.clone())
                    } else {
                        // If it doesnt, create new file info, cache it and return it
                        Ok(self.create_file_info(&path_to_file)?)
                    };
                }
            }

            bail!("Couldn't get the file in module")
        }
    }

    impl Debug for Project {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "Project [")?;
            writeln!(f, "\tRoot Path: {}", self.root_path)?;
            writeln!(f, "\tModules: [")?;
            for module in self.modules.iter() {
                writeln!(f, "\t\t(")?;
                writeln!(f, "\t\t\tModule: {}", module.0)?;
                writeln!(f, "\t\t\tInclude Paths: [")?;
                for include_path in module.1.iter() {
                    writeln!(f, "\t\t\t\t{},", include_path)?;
                }
                writeln!(f, "\t\t\t]")?;
                writeln!(f, "\t\t)")?;
            }
            writeln!(f, "\t]")?;
            writeln!(f, "\tfiles: {:?}", self.files)?;
            writeln!(f, "]")
        }
    }
}

use std::{fs::File, io::Write, path::Path, rc::Rc};

use anyhow::*;
use itertools::Itertools;

use crate::{node::Node, project::Project};

pub const CACHE_CONFIG_PATH: &str = "./.cache";

pub fn find_rec_deps(project_path: &str, entry_point: &str, output_file_path: &str) -> Result<()> {
    let mut project = Project::create(project_path)?;
    let entry_point_file_info = Rc::new(project.create_file_info(entry_point)?);

    let root_node = Node::create(&entry_point_file_info, None);

    let recursive_paths = Node::traverse(&root_node, &mut project);

    let mut file = File::create(Path::new(&output_file_path))?;

    for (file_name, paths) in recursive_paths.iter() {
        file.write_all(b"------------------------------------------------\n")?;

        file.write_all((format!("{}:\n", file_name)).as_bytes())?;

        let output_paths: Vec<&Vec<String>> = paths
            .iter()
            .sorted_by(|path1, path2| Ord::cmp(&path1.len(), &path2.len()))
            .collect();

        for path in output_paths {
            file.write_all(format!("\t{}\n", path.join("->")).as_bytes())?;
        }

        file.write_all("------------------------------------------------\n".as_bytes())?;
    }

    let mut config_file = File::create(CACHE_CONFIG_PATH)?;
    config_file
        .write_all(format!("{}\n{}\n{}", project_path, entry_point, output_file_path).as_bytes())?;

    Ok(())
}
