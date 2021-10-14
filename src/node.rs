use std::{
    rc::Rc,
    cell::RefCell,
    fmt::{
        Debug,
        Formatter,
    },
};
use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::{
    project::Project,
    file_info::FileInfo,
};

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

        let node = Rc::new(RefCell::new(Self {
            file_info: file_info.clone(),
            prev: prev.clone(),
            children: vec![],
            node_path,
        }));

        node.clone()
    }

    pub fn traverse(starting_node: &Rc<RefCell<Node>>, project: &mut Project)
                    -> HashMap<String, HashSet<Vec<String>>> {
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
            if current_children.len() <= 0 {
                // If the doesn't have children yet
                // Check if the file o the node actually has any includes
                let current_file_info = (*current).borrow().file_info.clone();
                if (*current_file_info).borrow().includes.len() > 0 {
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
            if let Some(unprocessed_child) = current_children.iter()
                .find(|&child| {
                    !(*(*child.clone()).borrow().file_info).borrow().processed
                }) {
                // If we find one, we check if it's not a recursive one
                let (is_unprocessed_child_recursive, file_name) = (*unprocessed_child.clone())
                    .borrow().is_recursive();
                if is_unprocessed_child_recursive {
                    // If it is recursive, it can be considered processed right away and we print
                    // out its path
                    (*(*unprocessed_child.clone()).borrow_mut().file_info)
                        .borrow_mut().processed = true;

                    let readable_path = (*unprocessed_child.clone()).borrow()
                        .readable_path();

                    let key = file_name.unwrap();

                    if let Some(path) =
                    recursive_paths.get_mut(key.as_str()) {
                        path.insert(readable_path.clone());
                    } else {
                        let mut set = HashSet::new();
                        set.insert(readable_path.clone());

                        recursive_paths.insert(key, set);
                    }

                    eprintln!(
                        "RECURSIVE PATH FOUND: {:?}",
                        readable_path
                    );
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

        let node_children = (*file_info).borrow().includes.iter()
            .filter_map(|include| {
                if let Some(include_file_info) = project.get_file(
                    include.clone(),
                    (*file_info).borrow().module.clone(),
                ) {
                    Some(Node::create(&include_file_info, Some(node.clone())))
                } else {
                    None
                }
            }).collect();

        node.borrow_mut().children = node_children;
    }

    fn is_recursive(&self) -> (bool, Option<String>) {
        let mut abs_paths = self.node_path.iter()
            .map(|file_info| (*file_info).borrow().abs_path.clone());

        if !abs_paths.all_unique() {
            (true, Some((*self.node_path.last().unwrap()).borrow().file_name.clone()))
        } else {
            (false, None)
        }
    }

    fn readable_path(&self) -> Vec<String> {
        self.node_path.iter().map(|node| {
            (*node).borrow().file_name.clone()
        }).collect()
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.file_info == other.file_info
            && self.prev == other.prev
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Node (")?;
        writeln!(f, "\tFile Info: {}", (*self.file_info).borrow().abs_path)?;
        writeln!(f, "\tPrevious Node: {}", match self.prev.clone() {
            Some(previous_node) => (*(*previous_node).borrow().file_info)
                .borrow().file_name.clone(),
            None => "None".to_owned()
        })?;
        writeln!(f, "\tChildren: {:?}", self.children.iter().map(|child| {
            (*(**child).borrow().file_info).borrow().file_name.clone()
        }).collect::<Vec<String>>())?;
        writeln!(f, "\tNode Path: {:?}", self.node_path)?;
        writeln!(f, ")")
    }
}