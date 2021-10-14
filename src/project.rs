use std::{
    rc::Rc,
    ops::Add,
    fs::File,
    path::Path,
    iter::FromIterator,
    io::{
        BufRead,
        BufReader,
    },
    fmt::{
        Debug,
        Formatter,
    },
    collections::{
        HashMap,
        HashSet,
    },
};
use std::cell::RefCell;

use substring::Substring;

use crate::file_info::FileInfo;

pub struct Project {
    pub root_path: String,
    pub modules: Vec<(String, Vec<String>)>,
    pub files: Vec<Rc<RefCell<FileInfo>>>,
    pub circular_dependency_paths: HashSet<Vec<String>>,
}

impl Project {
    pub fn create(project_path: String) -> Self {
        let cmake_lists_file = File::open(
            Path::new((project_path.clone() + "/CMakeLists.txt").as_str())
        ).expect("Failed to open CMakeLists.txt");

        let mut modules: HashMap<String, HashSet<String>> = HashMap::new();

        let cmake_lists_lines = BufReader::new(cmake_lists_file).lines();

        for cmake_lists_line in cmake_lists_lines {
            if let Ok(cll) = cmake_lists_line {
                let stripped_cll = cll.replace(" ", "");

                if stripped_cll.contains("include(") {
                    let include = stripped_cll
                        .replace("include(\"", "")
                        .replace("\")", "");

                    if !include.contains("includes") {
                        continue;
                    }

                    let include_cmake_file = File::open(
                        Path::new(include.clone().as_str())
                    )
                        .expect(format!("Couldn't open include file: {}", include).as_str());

                    let include_cmake_file_lines =
                        BufReader::new(include_cmake_file).lines();

                    for include_cmake_file_line in include_cmake_file_lines {
                        if let Ok(icfl) = include_cmake_file_line {
                            let stripped_ifl = icfl.replace(" ", "");

                            if stripped_ifl.contains("\"") {
                                let inc_folder = stripped_ifl
                                    .replace("\"", "")
                                    .replace("\t", "")
                                    .replace("\n", "");

                                if inc_folder.contains("Intermediate") {
                                    continue;
                                }

                                let start_ind = inc_folder
                                    .rfind("Engine/")
                                    .expect("Couldn't find 'Engine/' in the path");

                                let module = inc_folder
                                    .substring(
                                        start_ind,
                                        inc_folder.len(),
                                    ).replace("/Public", "")
                                    .replace("/Private", "");

                                if modules.contains_key(module.clone().as_str()) {
                                    modules
                                        .get_mut(module.clone().as_str())
                                        .unwrap()
                                        .insert(inc_folder);
                                } else {
                                    modules.insert(
                                        module.clone(),
                                        HashSet::from_iter([inc_folder]),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut res_modules: Vec<(String, Vec<String>)> = modules.iter()
            .map(|(module, include_paths)| (
                module.clone(),
                include_paths
                    .iter()
                    .map(|inc_path| inc_path.clone())
                    .collect::<Vec<String>>()
            )).collect();
        res_modules.sort_by(|(mod1, _inc1), (mod2, _inc2)| {
            Ord::cmp(&mod1.len(), &mod2.len())
        });

        Self {
            root_path: project_path.clone(),
            modules: res_modules,
            files: vec![],
            circular_dependency_paths: HashSet::new(),
        }
    }

    pub fn create_file_info(&mut self, abs_path: String) -> Rc<RefCell<FileInfo>> {
        let file_info = FileInfo::create(abs_path, &self.modules);

        self.files.push(file_info.clone());

        file_info
    }

    pub fn get_file(&mut self, partial_path: String, entry_module: String)
                    -> Option<Rc<RefCell<FileInfo>>> {
        // Check if root module actually exists
        let mut root_module = None;

        for modl in self.modules.clone() {
            if modl.0 == entry_module.clone() {
                root_module = Some(modl);
                break;
            }
        }

        // If it does
        if root_module.is_some() {
            let modl = root_module.clone().unwrap();

            if let Some(file) = self.get_file_in_module(
                modl.clone(),
                partial_path.clone(),
            ) {
                return Some(file);
            }
        }

        let other_modules: Vec<(String, Vec<String>)> =
            if let Some(root_mod) = root_module {
                self.modules.iter()
                    .filter(|(modl, _include_paths)|
                        modl.clone() != root_mod.0.clone()
                    )
                    .map(|modl| modl.clone())
                    .collect()
            } else {
                self.modules.clone()
            };

        for module in other_modules {
            if let Some(file) = self.get_file_in_module(
                module,
                partial_path.clone(),
            ) {
                return Some(file);
            }
        }

        None
    }

    fn get_file_in_module(
        &mut self,
        modl: (String, Vec<String>),
        partial_path: String,
    ) -> Option<Rc<RefCell<FileInfo>>> {
        // Check if any of the paths inside of the module are viable for the file we're looking
        // for
        for include_path in modl.1.iter() {
            // Concatenating the include path and partial path
            let path_to_file = include_path.clone()
                .add("/")
                .add(partial_path.as_str());

            // If path exists on the computer
            if Path::new(path_to_file.clone().as_str()).exists() {
                // Return cached file info if it exists
                return if let Some(file) = self.files.iter()
                    .find(|f| (*f).borrow().abs_path == path_to_file) {
                    Some(file.clone())
                } else {
                    // If it doesnt, create new file info, cache it and return it
                    Some(self.create_file_info(path_to_file))
                };
            }
        }

        None
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