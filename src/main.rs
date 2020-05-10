#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(seek_convenience)]
use std::{env, fs::File, io::{self, Write, Read}};
mod util;
mod pakinterface;

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
                let mut dump_file = File::create(file_path)?;
                dump_file.write_all(&child.data()?)?;
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
    //std::process::Command::new("paktool.exe").arg("01b_spacestation_decompressed.pak").spawn().unwrap();
    std::process::exit(0);
}

fn main() -> io::Result<()> {
    replacement_test();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Bad input");
    }
    let input_file = File::open(args[1].clone())?;
    let interface = pakinterface::PakInterface::open(input_file)?;

    //dump_csv(interface.get_root_node(), 0);
    dump_resource_info(interface.get_root_node(), 0);
    //dump_data(interface.get_root_node(), env::current_dir()?.to_str().unwrap().to_string() + "\\dump\\")?;
    Ok(())
}
