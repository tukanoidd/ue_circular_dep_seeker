use std::{fs::File, io::Write, path::Path, rc::Rc};

use gtk::{glib::Sender, prelude::*};
use relm4::{send, AppUpdate, Model, RelmApp, WidgetPlus, Widgets};

use itertools::Itertools;
use native_dialog::{FileDialog, MessageDialog, MessageType};

use ue_rec_deps_seeker::{node::Node, project::Project};

#[derive(Copy, Clone)]
enum ArgPath {
    Project,
    EntryPoint,
    OutputFile,
}

impl ArgPath {
    pub const fn label(&self) -> &'static str {
        match self {
            ArgPath::Project => "Project Path",
            ArgPath::EntryPoint => "Entry Point Path",
            ArgPath::OutputFile => "Output File Path",
        }
    }

    pub const fn example(&self) -> &'static str {
        match self {
            ArgPath::Project => "ex. /home/user/UnrealEngine",
            ArgPath::EntryPoint => "ex. /home/user/UnrealEngine/Engine/Source/../../Math.h",
            ArgPath::OutputFile => "ex. /home/user/UnrealEngine/rec_deps.txt",
        }
    }
}

enum AppMsg {
    Choose(ArgPath),
    Update((ArgPath, String)),
    StartAlgo,
}

#[tracker::track]
#[derive(Default)]
struct AppModel {
    project_path: Option<String>,
    entry_point: Option<String>,
    output_file: Option<String>,
    was_successful: Option<bool>,
}

impl AppModel {
    fn all_paths(&self) -> (bool, Option<String>) {
        if self.project_path.is_some() && self.entry_point.is_some() && self.output_file.is_some() {
            (true, None)
        } else {
            let mut output = String::new();
            let mut doesnt_exist = vec![];

            if self.project_path.is_none() {
                doesnt_exist.push("Project");
            }

            if self.entry_point.is_none() {
                doesnt_exist.push("Entry Point");
            }

            if self.output_file.is_none() {
                doesnt_exist.push("Output File");
            }

            if doesnt_exist.len() == 1 {
                output += &format!("{} was not set!", doesnt_exist.first().unwrap());
            } else {
                output += &format!("{} were not set!", doesnt_exist.join(" & "));
            }

            (false, Some(output))
        }
    }

    fn unwrap_all(&self) -> (String, String, String) {
        (
            self.project_path.clone().unwrap(),
            self.entry_point.clone().unwrap(),
            self.output_file.clone().unwrap(),
        )
    }

    fn paths_arr(&self) -> [&Option<String>; 3] {
        [&self.project_path, &self.entry_point, &self.output_file]
    }
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(
        &mut self,
        msg: Self::Msg,
        _components: &Self::Components,
        _sender: Sender<Self::Msg>,
    ) -> bool {
        self.reset();

        match msg {
            AppMsg::Choose(path) => match path {
                ArgPath::Project => {
                    let mut location = "~".to_string();
                    if self.project_path.is_some() {
                        location = self.project_path.clone().unwrap();
                    }

                    let path = FileDialog::new()
                        .set_location(&location)
                        .show_open_single_dir();

                    match path {
                        Ok(path) => {
                            if let Some(path) = path {
                                self.set_project_path(path.to_str().map(|p| p.to_string()));
                            }
                        }
                        Err(error) => {
                            println!("Couldn't get project path: {}", error)
                        }
                    }
                }
                ArgPath::EntryPoint => {
                    let mut location = "~".to_string();
                    if self.entry_point.is_none() && self.project_path.is_some() {
                        location = self.project_path.clone().unwrap();
                    }

                    let path = FileDialog::new()
                        .set_location(&location)
                        .show_open_single_file();

                    match path {
                        Ok(path) => {
                            if let Some(path) = path {
                                self.set_entry_point(path.to_str().map(|p| p.to_string()));
                            }
                        }
                        Err(error) => {
                            println!("Couldn't get entry point path: {}", error)
                        }
                    }
                }
                ArgPath::OutputFile => {
                    let mut location = "~".to_string();
                    if self.output_file.is_none() && self.project_path.is_some() {
                        location = self.project_path.clone().unwrap();
                    }

                    let path = FileDialog::new()
                        .set_location(&location)
                        .show_save_single_file();

                    match path {
                        Ok(path) => {
                            if let Some(path) = path {
                                self.set_output_file(path.to_str().map(|p| p.to_string()));
                            }
                        }
                        Err(error) => {
                            println!("Couldn't get output file path: {}", error)
                        }
                    }
                }
            },
            AppMsg::Update((path, path_str)) => match path {
                ArgPath::Project => self.set_project_path(Some(path_str)),
                ArgPath::EntryPoint => self.set_entry_point(Some(path_str)),
                ArgPath::OutputFile => self.set_output_file(Some(path_str)),
            },
            AppMsg::StartAlgo => match self.all_paths() {
                (false, Some(message)) => {
                    println!("{}", message);
                    return false;
                }
                (true, None) => {
                    let (project_path, entry_point, output_file_path) = self.unwrap_all();

                    let mut project = Project::create(&project_path);
                    let entry_point_file_info = Rc::new(project.create_file_info(&entry_point));

                    let root_node = Node::create(&entry_point_file_info, None);

                    let recursive_paths = Node::traverse(&root_node, &mut project);

                    let mut file = File::create(Path::new(&output_file_path))
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

                        file.write_all(
                            "------------------------------------------------\n".as_bytes(),
                        )
                        .expect("Couldn't write to the output file");
                    }

                    self.set_was_successful(Some(true));

                    let open_file = MessageDialog::new()
                        .set_type(MessageType::Info)
                        .set_title("Success!")
                        .set_text("Do you want to open the file?")
                        .show_confirm()
                        .unwrap();

                    if open_file {
                        open::that(&output_file_path)
                            .expect("Couldn't open the file with the default text editor!");
                    }
                }
                _ => panic!("Something went horribly wrong with getting info about paths"),
            },
        }

        true
    }
}

struct AppWidgets {
    window: gtk::ApplicationWindow,
    entries: [gtk::Entry; 3],
    success_message: gtk::Label,
}

impl Widgets<AppModel, ()> for AppWidgets {
    type Root = gtk::ApplicationWindow;

    fn init_view(_model: &AppModel, _components: &(), sender: Sender<AppMsg>) -> Self {
        let window = gtk::ApplicationWindow::builder()
            .default_height(500)
            .default_width(500)
            .title("UE Recursive Dependecnies Seeker")
            .build();
        let main_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .build();
        main_container.set_margin_all(5);

        window.set_child(Some(&main_container));

        let entries =
            [ArgPath::Project, ArgPath::EntryPoint, ArgPath::OutputFile].map(|entry_type| {
                let btn_sender = sender.clone();
                let entry_sender = sender.clone();

                let hbox = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(5)
                    .build();
                hbox.set_margin_all(5);

                let label = gtk::Label::new(Some(entry_type.label()));

                let entry = gtk::Entry::builder()
                    .editable(true)
                    .placeholder_text(entry_type.example())
                    .build();

                let button = gtk::Button::builder().label("...").build();

                hbox.append(&label);
                hbox.append(&entry);
                hbox.append(&button);

                entry.connect_changed(move |e| {
                    send!(
                        entry_sender,
                        AppMsg::Update((entry_type, e.buffer().text()))
                    )
                });
                button.connect_clicked(move |_| send!(btn_sender, AppMsg::Choose(entry_type)));

                main_container.append(&hbox);

                entry
            });

        let start_algo_button = gtk::Button::builder().label("Start Algorithm").build();
        let success_message = gtk::Label::new(Some("Run Algo"));

        main_container.append(&start_algo_button);
        main_container.append(&success_message);

        start_algo_button.connect_clicked(move |_| send!(sender, AppMsg::StartAlgo));

        Self {
            window,
            entries,
            success_message,
        }
    }

    fn root_widget(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(&mut self, model: &AppModel, _sender: Sender<AppMsg>) {
        let vals = model.paths_arr();
        let entries_changed = [
            model.changed(AppModel::project_path()),
            model.changed(AppModel::entry_point()),
            model.changed(AppModel::output_file()),
        ];

        for (i, val) in vals.iter().enumerate() {
            if let Some(val) = val {
                if entries_changed[i] {
                    println!("{}", val);
                    self.entries[i].set_text(val);
                }
            }
        }

        if model.changed(AppModel::was_successful()) {
            match model.was_successful {
                Some(was_successful) => self.success_message.set_text(match was_successful {
                    true => "Success!",
                    false => "Failed!",
                }),
                None => self.success_message.set_text("Run Algo"),
            }
        }
    }
}

fn main() {
    let model = AppModel::default();

    let app = RelmApp::new(model);
    app.run();
}
