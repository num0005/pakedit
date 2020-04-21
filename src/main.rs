#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(seek_convenience)]
use std::{env, fs::File, io::{self}};
mod util;
mod pakinterface;

fn dump_csv(node: &pakinterface::ResourceNode, depth: usize)
{
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

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("No pak files!");
    }
    let input_file = File::open(args[1].clone())?;
    let root_node = pakinterface::read_file(input_file)?;

    dump_csv(&root_node, 0);

    Ok(())
}
