#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(seek_convenience)]
#![feature(seek_stream_len)]
use std::{env, fs::File, io::{self, Write, Read}, time::{Instant}};
mod util;
mod pakinterface;
use pakinterface::{PakInterface, ResourceNode, ResourceChild};

#[allow(dead_code)]
fn dump_csv(node: &pakinterface::ResourceNode, depth: usize) {
    for child in node.children() {
        match child.contents() {
            pakinterface::ResourceType::Node(child_node) =>
            {
                dump_csv(child_node, depth + 1);
            },
            _ => {}
        }

        println!("{}{}, {}", "- ".repeat(depth), child.name(), child.size());
    }
}

#[allow(dead_code)]
fn dump_resource_info(node: &pakinterface::ResourceNode, depth: usize) {
    for child in node.children() {
        let extra: String;
        match child.contents() {
            pakinterface::ResourceType::Node(child_node) =>
            {
                dump_resource_info(child_node, depth + 1);
                extra = format!("{:?}", child_node.header().meta_data());
            },
            pakinterface::ResourceType::Resource(header) => 
            {
                extra = format!("{:?}", header.meta_data());
            }
            pakinterface::ResourceType::Link(_) => {continue},
            _ => {continue},
        }

        println!("{}{}, {} {}", "- ".repeat(depth), child.name(), child.offset(), extra);
    }
}

fn dump_file(path: String, child: &ResourceChild) -> io::Result<()> {
    let mut dump_file = File::create(&path)?;
    dump_file.write_all(&child.data()?)?;
    Ok(())
}

#[allow(dead_code)]
fn dump_data(node: &pakinterface::ResourceNode, path: String) -> io::Result<()> {
    for child in node.children() {
        let file_name = child.name().rsplit(">\\").next().unwrap().rsplit(":").next().unwrap();
        match child.contents() {
            pakinterface::ResourceType::Node(child_node) =>
            {
                dump_data(child_node, path.clone() + file_name.to_string().split(".").next().unwrap() + "\\")?;
            },
            pakinterface::ResourceType::Resource(_) | pakinterface::ResourceType::Data =>
            {
                let file_path_string = path.clone() + file_name;
                let file_path = std::path::Path::new(&file_path_string);
                std::fs::create_dir_all(file_path.parent().unwrap())?;
                dump_file(file_path_string, child)?;
            },
            _ => {} // ignore links
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn replace_script(node: &mut pakinterface::ResourceNode) {
    const FILE: &str = "ingame_cinematic_mgr.ssl";

    println!("Loading replacement script from \"{}\"", FILE);
    let mut replacement = File::open(FILE).unwrap();
    let mut data = Vec::new();
    replacement.read_to_end(&mut data).unwrap();

    for child in node.children_mut() {
        match child.contents_mut() {
            pakinterface::ResourceType::Node(child_node) => {
                replace_script(child_node);
            },
            pakinterface::ResourceType::Data =>
            {
                if child.name().contains(FILE) {
                    println!("Replacing file \"{}\"", child.name());
                    child.set_data(data.clone());
                }
            }
            _=> {}
        }
    }
}

#[allow(dead_code)]
fn replacement_test() {
    env::set_current_dir("H:\\SteamLibrary\\steamapps\\common\\Halo The Master Chief Collection Flighting\\halo2\\preload\\paks").unwrap();
    let input_file = File::open("01b_spacestation_og.pak").unwrap();
    let mut interface = pakinterface::PakInterface::open(input_file).unwrap();
    //replace_script(interface.get_root_node_mut());
    //interface.save(File::create("01b_spacestation_decompressed.pak").unwrap()).unwrap();
    std::process::Command::new("paktool.exe").arg("01b_spacestation_decompressed.pak").spawn().unwrap();
    std::process::exit(0);
}

#[derive(Debug, Default)]
struct UserInterface {
    pak_file: Option<PakInterface>,
    filter: String,
    node: Vec<usize>, // implemented like this cause lifetimes are hard
    exit: bool,
}

impl UserInterface {

    fn get_node_recursive<'a>(parent: &'a mut ResourceNode, left: &[usize]) -> &'a mut ResourceNode {
        if let pakinterface::ResourceType::Node(node) = parent.children_mut()[left[0]].contents_mut() {
            if left.len() > 1 {
                Self::get_node_recursive(node, &left[1..])
            } else {
                node
            }
        } else {
            panic!("internal error");
        }
    }

    fn get_node(&mut self) -> Option<&mut ResourceNode> {
        if let Some(pak) = &mut self.pak_file {
            let root_node = pak.get_root_node_mut();
            if self.node.len() == 0 {
                Some(root_node)
            } else {
                Some(Self::get_node_recursive(root_node, &self.node[..]))
            }
        } else {
            None
        }
    }

    fn find_child(&mut self, tag: &str) -> Option<&mut ResourceChild> {
        if let Some(node) = self.get_node() {
            for child in node.children_mut() {
                if child.name() == tag {
                    return Some(child)
                }
            }
        }
        None
    }

    fn open_pak(&mut self, path: &str) {
        let start = Instant::now();
        match File::open(path) {
            Ok(file) => {
                match pakinterface::PakInterface::open(file) {
                    Ok(interface) => {
                        self.pak_file = Some(interface);
                        println!("Pack file opened in {} seconds", start.elapsed().as_secs());
                    }
                    Err(error) => println!("Failed to read pack file, {}", error)
                }
            }
            Err(error) => println!("Failed to open pack file, {}", error)
        }
    }

    fn open_pak_node(&mut self, path: &str) {
        let children = self.get_node().unwrap().children();
        for i in 0..children.len() {
            if children[i].name() == path {
                if let pakinterface::ResourceType::Node(_) = children[i].contents() {
                    self.node.push(i);
                    return;
                }
            }
        }
        println!("Failed to open pack node!");
    }

    fn list(&mut self) {
        let filter = self.filter.clone();
        if let Some(node) = self.get_node() {
            for child in node.children() {
                if child.name().starts_with(&filter) {
                    println!("{}, {}", child.name(), child.contents());
                }
            }
        } else {
            println!("No pack file loaded!");
        }
    }

    fn export(&mut self, tag: &str) {
        if let Some(child) = self.find_child(tag) {
            let save_name = tag.rsplit("\\").next().unwrap();
            println!("Saving file as \"{}\"", save_name);
            if let Err(error) = dump_file(save_name.to_string(), child) {
                println!("Failed to export file {}", error);
            }
        } else {
            println!("No such resource!");
        }
    }

    fn run(&mut self) -> io::Result<()> {
        while !self.exit {
            if self.filter.is_empty() {
                print!(">");
            } else {
                print!("[{}]>", self.filter);
            }
            std::io::stdout().flush()?;
            
            let mut line : String = text_io::read!("{}\n");
            line = line.replace("\r", "");
            if let Some(input) = shlex::split(line.as_str()) {
                if input.len() < 1 {
                    continue;
                }
                match input[0].to_lowercase().as_str() {
                    "open" => {
                        if input.len() == 2 {
                            if self.pak_file.is_none() {
                                self.open_pak(input[1].as_str())
                            } else {
                                self.open_pak_node(input[1].as_str())
                            }
                        } else {
                            println!("Open takes 1 arg");
                        }
                    },
                    "export" => {
                        if input.len() == 2 {
                            self.export(input[1].as_str());
                        } else {
                            println!("Export takes 1 arg");
                        }
                    },
                    "list" => self.list(),
                    "close" => self.close(),
                    "exit" | "quit" => self.exit = true,
                    "print_csv" => {
                        if let Some(node) = self.get_node() {
                            dump_csv(node, 0);
                        } else {
                            println!("No pack file loaded!");
                        }
                    }
                    "filter" => {
                        if input.len() == 2 {
                            self.filter = input[1].clone();
                        } else {
                            self.filter = String::default();
                        }
                    },
                    _=> println!("Unknown command")
                }
            } else {
                println!("Unable to parse line {}", line);
            }
        }
        Ok(())
    }

    fn close(&mut self) {
        if self.node.len() == 0 {
            self.pak_file = None;
        } else {
            self.node.pop();
        }
    }
}

fn main() -> io::Result<()> {
    let mut interface = UserInterface::default();
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        interface.open_pak(args[1].as_str());
    }

    interface.run()?;
    Ok(())
}
