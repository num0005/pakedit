use std::{fs::File, io::{self, Read, Seek, SeekFrom, Write}, rc::{Weak, Rc}, cell::RefCell};
use getset::{Getters};
use crate::util;

pub const RESOURCE_MAGIC: u32 = util::u32_from_str("RES1");
pub const TYPE_PACKAGE: [u8; 0xC] = [0x70, 0x61, 0x6B, 0, 0, 0, 0, 0, 0, 0, 0, 0];


#[derive(Debug)]
pub enum ResourceType {
    Data,
    /// Index to link node
    Link(usize),
    Node(ResourceNode)
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Data
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
enum NodeModifiedState {
    /// No change
    Clean,
    /// Only header needs to be changed
    Header,
    /// Data modified but only appeneded (existing data can be copied)
    Append,
    /// Rebuild whole node
    Full
}

impl Default for NodeModifiedState {
    fn default() -> Self {
        NodeModifiedState::Clean
    }
}

#[derive(Debug, Default, Getters)]
pub struct ResourceChild {

    /* Data we read from the pak */

    #[getset(get = "pub")]
    name: String,
    offset: u64,
    #[getset(get = "pub")]
    size: u32,
    is_link: bool,

    /* Public info */

    #[getset(get = "pub", get_mut = "pub")]
    contents: ResourceType,

    /* Implementation detail */

    new_data: Option<Vec<u8>>,
    node_base: u64,
    meta_data_dirty: bool,
    /// are there active links to this node?
    has_active_links: bool,
    is_new_entry: bool,

    input_file: Weak<std::cell::RefCell<File>>
}

#[allow(dead_code)]
impl ResourceChild {
    pub fn contents_mut(&mut self) -> &mut ResourceType {
        &mut self.contents
    }

    /// Rename the child
    pub fn rename(&mut self, name: String) {
        self.name = name;
        self.meta_data_dirty = true;
    }

    /// Get raw data
    pub fn data(&self) -> io::Result<Vec<u8>> {
        match &self.new_data {
            Some(data) => Ok(data.to_vec()),
            None => {
                let mut data = vec![0u8; self.size as usize];
                // value is still valid if we exist
                let file_ref = self.input_file.upgrade().unwrap();
                let mut file = file_ref.borrow_mut();
                file.seek(SeekFrom::Start(self.offset + self.node_base))?;
                file.read_exact(&mut data)?;
                Ok(data)
            }
        }
    }

    /// Set raw data
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.size = data.len() as u32;
        self.new_data = Some(data);
    }
}

#[derive(Debug, Default, Getters)]
pub struct ResourceNode {
    /* Data we read from the pak */

    #[getset(get = "pub")]
    resource_type: [u8; 0xC],
    #[getset(get = "pub")]
    type_uuid: u128,
    unk0: u32,
    unk1: [u8; 0x10],
    unk2: [u8; 0x10],

    /* Public info */

    #[getset(get = "pub")]
    children: Vec<ResourceChild>,

    /* Implementation detail */

    state: NodeModifiedState,
    data_offset: u64,
    header_len: u64,
    data_len: u64,
    input_file: Weak<RefCell<File>>,
}

impl ResourceNode {
    pub fn children_mut(&mut self) -> &mut[ResourceChild] {
        &mut self.children[..]
    }

    fn update_state(&mut self) {
        for child in &mut self.children {
            if child.meta_data_dirty {
                if self.state == NodeModifiedState::Clean {
                    self.state = NodeModifiedState::Header
                }
            }
            match &mut child.contents {
                ResourceType::Data => {
                    if child.new_data.is_some() {
                        if child.is_new_entry && self.state != NodeModifiedState::Full {
                            self.state = NodeModifiedState::Append;
                        } else {
                            self.state = NodeModifiedState::Full;
                        }
                    }
                },

                ResourceType::Node(node) => {
                    node.update_state();
                    if node.state != NodeModifiedState::Clean {
                        self.state = NodeModifiedState::Full;
                    }
                },

                _=> {}
            }
        }
    }

    fn save(&mut self, mut file : &File) -> io::Result<u64> {
        let offset_start = file.stream_position()?;
        let child_count = self.children.len();

        // write basic header

        util::write_u32(file, RESOURCE_MAGIC)?;
        file.write_all(&self.resource_type)?;
        util::write_u128(file, self.type_uuid)?;
        util::write_u32(file, self.unk0)?;
        file.write_all(&self.unk1)?;
        file.write_all(&self.unk2)?;
        util::write_u8(file, 0x01)?;

        util::write_u32(file, child_count as u32)?;
        file.write_all(&[0x04, 0x00, 0x00, 0x00, 0x01])?;

        // write file list

        for child in &self.children {
            let name = child.name.as_bytes();
            util::write_u32(file, name.len() as u32)?;
            file.write_all(name)?;
        }

        util::write_u8(file, 0x01)?;

        let offset_section = file.stream_position()?;

        // nasty but the easiest way to do this in rust
        let offset_data_temp = vec![0u8; child_count * 8 + 1 + child_count * 4];
        file.write_all(&offset_data_temp)?;

        util::write_u8(file, 0x01)?;

        for child in &self.children {
            match child.contents {
                ResourceType::Link(_) => { util::write_u32(file, 1) }
                _=> {  util::write_u32(file, 0) }
            }?
        }

        let input_file_ref = self.input_file.upgrade().unwrap();

        if self.state == NodeModifiedState::Clean {
            let mut input_file = input_file_ref.borrow_mut();
            input_file.seek(SeekFrom::Start(self.data_offset))?;
            util::copy_data(&input_file, file, self.data_len as usize)?;
        } else { // todo use the other state info or remove it

            // write child data
            for child in &mut self.children {
                let new_offset = file.stream_position()? - offset_start;
                match &mut child.contents {
                    ResourceType::Data => {
                        match &child.new_data {
                            Some(data) => { file.write_all(data)?; }
                            None => {
                                let mut input_file = input_file_ref.borrow_mut();
                                input_file.seek(SeekFrom::Start(child.node_base + child.offset))?;
                                util::copy_data(&input_file, file, child.size as usize)?;
                            }
                        }
                    },
                    ResourceType::Node(node) => {
                        if node.state != NodeModifiedState::Clean {
                            child.size = node.save(&file)? as u32;
                        } else {
                            let mut input_file = input_file_ref.borrow_mut();
                            input_file.seek(SeekFrom::Start(child.node_base + child.offset))?;
                            util::copy_data(&input_file, file, child.size as usize)?;
                        }
                    }
                    _=> continue
                }
                child.offset = new_offset;
            }

            // update links
            for child_index in 0..child_count {
                if let ResourceType::Link(idx) = self.children[child_index].contents {
                    self.children[child_index].offset = self.children[idx].offset;
                }
            }
        }

        let end = file.stream_position()?;

        file.seek(SeekFrom::Start(offset_section))?;

        for child in &self.children {
            util::write_u64(file, child.offset)?;
        }

        util::write_u8(file, 0x01)?;

        for child in &self.children {
            util::write_u32(file, child.size)?;
        }

        file.seek(SeekFrom::Start(end))?;
        Ok(end - offset_start)
    }
}

fn read_node(file_ref: Weak<RefCell<File>>, mut file: &File, shared_hack: bool) ->io::Result<ResourceNode> {
    // children are relative to node base
    let node_base = file.stream_position()?;

    let magic = util::read_u32(file)?;
    if magic != RESOURCE_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Bad resource node magic @ {}",  node_base)
        ))
    }
    let mut node = ResourceNode::default();
    node.input_file = file_ref.clone();
    file.read_exact(&mut node.resource_type)?;
    node.type_uuid = util::read_u128(file)?;
    node.unk0 = util::read_u32(file)?;
    file.read_exact(&mut node.unk1[..])?;
    file.read_exact(&mut node.unk2[..])?;
    assert_eq!(node.unk2, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
    assert_eq!(util::read_u8(file)?, 0x01);

    let child_count = util::read_u32(file)?;
    assert_eq!(util::read_u32(file)?, 4);
    assert_eq!(util::read_u8(file)?, 0x01);

    for _ in 0..child_count {
        let mut child = ResourceChild::default();
        child.node_base = node_base;
        child.input_file = file_ref.clone();
        let string_len = util::read_u32(file)? as usize;
        let mut data = vec![0u8; string_len];
        file.read_exact(&mut data)?;

        child.name = String::from_utf8(data).unwrap(); // don't care enough to handle

        node.children.push(child);
    }
    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count as usize {
        node.children[child_index].offset = util::read_u64(file)?;
    }

    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count as usize {
        node.children[child_index].size = util::read_u32(file)?;
    }

    assert_eq!(util::read_u8(file)?, 0x01);

    for child_index in 0..child_count as usize {
        node.children[child_index].is_link = util::read_u32(file)? > 0;
    }

    node.data_offset = file.stream_position()?;
    node.header_len = node.data_offset - node_base;

    if node.resource_type == TYPE_PACKAGE && !shared_hack {
        for child_index in 0..child_count as usize {
            if !node.children[child_index].is_link {
                let node_file_offset = node_base + node.children[child_index].offset;
                file.seek(SeekFrom::Start(node_file_offset))?;
                let mut child_node = read_node(file_ref.clone(), file, shared_hack)?;
                child_node.data_len = node.children[child_index].size as u64 - (child_node.data_offset - node_file_offset);
                node.children[child_index].contents = ResourceType::Node(child_node);
            } else {
                for other_child_index in 0..child_count as usize {
                    if !node.children[other_child_index].is_link &&
                            node.children[other_child_index].offset == node.children[child_index].offset {
                        node.children[child_index].contents = ResourceType::Link(other_child_index);
                        node.children[other_child_index].has_active_links = true; // prevent it from being moved or deleted
                    }
                }
            }
        }
    }

    Ok(node)
}

#[derive(Debug)]
pub struct PakInterface {
    input_file: std::rc::Rc<RefCell<File>>,
    shared_hack: bool,
    root_node: ResourceNode
}

#[allow(dead_code)]
impl PakInterface {
    pub fn get_root_node_mut(&mut self) -> &mut ResourceNode {
        &mut self.root_node
    }

    pub fn get_root_node(&self) -> &ResourceNode {
        &self.root_node
    }

    pub fn open(file : File, shared_hack : bool) -> io::Result<PakInterface> {
        let input_file = Rc::new(RefCell::new(file));
        let mut root_node = read_node(Rc::downgrade(&input_file), &input_file.borrow_mut(), shared_hack)?;
        root_node.data_len = input_file.borrow_mut().stream_len()? - root_node.data_offset;
        Ok(PakInterface { input_file: input_file, root_node: root_node, shared_hack: shared_hack })
    }

    pub fn save(&mut self, file : File) -> io::Result<()> {
        self.root_node.update_state();
        self.root_node.save(&file)?;
        Ok(())
    }
}
