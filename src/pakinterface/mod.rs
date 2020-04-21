use std::{fs::File, io::{self, Read, Seek, SeekFrom, Write}};
use crate::util;

pub const RESOURCE_MAGIC: u32 = util::u32_from_str("RES1");
pub const TYPE_PACKAGE: [u8; 0xC] = [0x70, 0x61, 0x6B, 0, 0, 0, 0, 0, 0, 0, 0, 0];

#[derive(Clone, Debug)]
pub enum ResourceContents {
    None,
    RawData(Vec<u8>),
    Node(ResourceNode)
}

impl Default for ResourceContents {
    fn default() -> ResourceContents {
        ResourceContents::None
    }
}

#[derive(Clone, Debug, Default)]
pub struct ResourceChild {
    pub name: String,
    pub offset: u64, // relative to node base
    pub size: u32,
    pub is_link: bool,
    pub contents: ResourceContents
}

#[derive(Clone, Debug, Default)]
pub struct ResourceNode {
    pub resource_type: [u8; 0xC],
    pub type_uuid: u128,
    unk1: util::StaticArray<u8, 0x25>,
    pub children: Vec<ResourceChild>
}

fn read_node(mut file : &File) ->io::Result<ResourceNode> {
    // children are relative to node base
    let node_base = file.stream_position()?;

    let magic = util::read_u32(file)?;
    if magic != RESOURCE_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Bad resource node magic @ {}",  node_base)
        ))
    }
    let mut node = ResourceNode::default();
    file.read_exact(&mut node.resource_type)?;
    node.type_uuid = util::read_u128(file)?;
    file.read_exact(&mut node.unk1[..])?;

    let child_count = util::read_u32(file)?;
    let mut unknown: [u8; 5] = Default::default();
    file.read_exact(&mut unknown)?;
    assert_eq!(unknown, [0x04, 0x00, 0x00, 0x00, 0x01]);

    for _ in 0..child_count {
        let mut child = ResourceChild::default();
        let string_len = util::read_u32(file)? as usize;
        let mut data = vec![0u8; string_len];
        file.read_exact(&mut data)?;

        child.name = String::from_utf8(data).unwrap(); // don't care enough to handle

        node.children.push(child);
    }
    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count {
        node.children[child_index as usize].offset = util::read_u64(file)?;
    }

    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count {
        node.children[child_index as usize].size = util::read_u32(file)?;
    }

    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count {
        node.children[child_index as usize].is_link = util::read_u32(file)? > 0;
    }

    for child in &mut node.children {
        if child.is_link {
            continue;
        }

        file.seek(SeekFrom::Start(node_base + child.offset))?;
        if node.resource_type == TYPE_PACKAGE {
            child.contents = ResourceContents::Node(read_node(file)?);
        } else {
            let mut data = vec![0u8; child.size as usize];
            file.read_exact(&mut data)?;
            child.contents = ResourceContents::RawData(data);
        }
    }

    Ok(node)
}

pub fn read_file(file : File) -> io::Result<ResourceNode> {
    read_node(&file)
}
