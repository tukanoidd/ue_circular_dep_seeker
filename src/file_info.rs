use std::{
    rc::Rc,
    fs::File,
    path::Path,
    cell::RefCell,
    fmt::{
        Debug,
        Display,
        Formatter,
    },
    io::{
        BufRead,
        BufReader,
    }
};

#[derive(Eq, PartialEq, Hash)]
pub enum FileType {
    Header,
    Source,
    Inline
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Header => write!(f, "Header"),
            FileType::Source => write!(f, "Source"),
            FileType::Inline => write!(f, "Inline")
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
    pub fn create(abs_path: String, modules: &Vec<(String, Vec<String>)>) -> Rc<RefCell<FileInfo>> {
        let file = File::open(Path::new(abs_path.as_str()))
            .expect(format!("Couldn't open a file at: {}", abs_path).as_str());

        let file_name = abs_path.split("/").last().unwrap();
        let file_type_str = file_name.split(".").last().unwrap();

        let file_type = match file_type_str {
            "h" | "hpp" => FileType::Header,
            "c" | "cpp" => FileType::Source,
            "inl" => FileType::Inline,
            _ => panic!("{}", format!("File type is not supported: '{}'", file_type_str))
        };

        let file_lines = BufReader::new(file).lines();

        let mut includes = vec![];

        for line in file_lines {
            if let Ok(mut l) = line {
                if l.contains("#include") {
                    if l.contains(".generated.") || l.contains(".gen.") {
                        continue;
                    }

                    l = l.trim().to_owned();

                    let l_split = l.split(" ");

                    includes.push(
                        l_split.last().unwrap()
                            .replace("\"", "").to_owned()
                    );
                }
            }
        }

        let module = modules.iter().rfind(|(modl, _include_paths)| {
            abs_path.contains(modl.as_str())
        })
            .expect(format!("Couldn't find the module of the file: {}", abs_path).as_str())
            .0.clone();

        Rc::new(RefCell::new(Self {
            abs_path: abs_path.clone(),
            file_name: file_name.to_owned(),
            module: module.clone(),
            file_type,
            includes,
            processed: false
        }))
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