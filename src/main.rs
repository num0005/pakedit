#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(seek_convenience)]
use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}};
mod util;
mod pakinterface;

fn dump_csv(node: &pakinterface::ResourceNode, depth: usize) {
    for child in &node.children {

        match &child.contents {
            pakinterface::ResourceContents::Node(child_node) =>
            {
                dump_csv(child_node, depth + 1);
            },
            _ => {}
        }

        println!("{}{}, {}", "- ".repeat(depth), child.name, child.size);
    }
}

fn dump_data(mut file: &File, node: &pakinterface::ResourceNode, path: String) {
    for child in &node.children {
        let file_name = child.name.rsplit(">\\").next().unwrap().rsplit(":").next().unwrap();
        match &child.contents {
            pakinterface::ResourceContents::Node(child_node) =>
            {
                dump_data(file, child_node, path.clone() + file_name.to_string().split(".").next().unwrap() + "\\");
            },
            pakinterface::ResourceContents::RawData(info) =>
            {
                let file_path_string = path.clone() + file_name;
                let file_path = std::path::Path::new(&file_path_string);
                std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
                let mut dump_file = File::create(file_path).unwrap();
                
                let mut buffer = vec![0u8; info.size];
                file.seek(SeekFrom::Start(info.offset)).unwrap();
                file.read_exact(&mut buffer);
                dump_file.write_all(&buffer).unwrap();
            },
            _ => {} // ignore links
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("No pak files!");
    }
    let input_file = File::open(args[1].clone())?;
    let root_node = pakinterface::read_file(&input_file, args[1].contains("shared"))?;

    //dump_csv(&root_node, 0);
    dump_data(&input_file, &root_node, env::current_dir()?.to_str().unwrap().to_string() + "\\dump\\");
    Ok(())
}
