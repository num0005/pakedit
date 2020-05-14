use std::{fs::File, io, rc::{Weak, Rc}, cell::{RefCell}};
use getset::{Getters};
use crate::util;
use util::BinaryStream;
mod pak_io;
use pak_io::UncompressedFile;

pub const RESOURCE_MAGIC: u32 = util::u32_from_str("RES1");
pub const NODE_CLASSES: [&str; 3] = ["pak", "ssl_block", "cache_block"];


#[derive(Debug)]
pub enum ResourceType {
    /// Raw data with no headers or anything
    Data,
    /// Index to link node
    Link(usize),
    /// Node containing other data
    Node(ResourceNode),
    /// Data with a resource header
    Resource(ResourceHeader)
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Data
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::Data => write!(f, "Raw Data"),
            Self::Link(_) => write!(f, "Link"),
            Self::Node(_) => write!(f, "Node"),
            Self::Resource(_) => write!(f, "Resource Data"),
        }
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
    #[getset(get = "pub")]
    offset: u64,
    #[getset(get = "pub")]
    size: u32,

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

    input_file: Weak<std::cell::RefCell<UncompressedFile>>
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
    /// Will panic if not Data or Resource
    pub fn data(&self) -> io::Result<Vec<u8>> {
        let read_offset;
        match &self.contents {
            ResourceType::Data => read_offset = 0,
            ResourceType::Resource(header) => read_offset = header.size,
            _=> panic!("Bad contents")
        };
        
        match &self.new_data {
            Some(data) => Ok(data.to_vec()),
            None => {
                let mut data = vec![0u8; (self.size - read_offset) as usize];
                // value is still valid if we exist
                let file_ref = self.input_file.upgrade().unwrap();
                let mut file = file_ref.borrow_mut();
                file.seek(self.offset + self.node_base + read_offset as u64)?;
                file.read_bytes(&mut data)?;
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
#[getset(get = "pub")]
pub struct MetaData {
    unk0: u32,
    unk1: u128,
    name_data: Vec<u8>
}

#[derive(Debug, Default, Getters)]
#[getset(get = "pub")]
pub struct ResourceHeader {
    /* Read from pak */

    class: String,
    uuid: u128,
    unk0: u32,
    meta_data: Vec<MetaData>,

    /* Implementation detail */
    #[getset(get)]
    base: u64,
    size: u32
}

impl ResourceHeader {
    /// Returns the header if a valid one exists with the file pointer pointing 
    /// just beyond the end of the header, on error file pointer is undefined
    pub fn read<T: BinaryStream>(file : &mut T) -> io::Result<Option<ResourceHeader>> {
        let magic = file.read_u32()?;
        if magic != RESOURCE_MAGIC {
            return Ok(None);
        }

        let mut header = ResourceHeader::default();
        header.base = file.position()? - 4;

        let mut resource_class = [0u8; 0x20];
        file.read_bytes(&mut resource_class)?;
        let len = util::string_length(&resource_class);
        header.class = String::from_utf8(resource_class[..len].to_vec()).unwrap();

        header.uuid = file.read_u128()?;
        header.unk0 = file.read_u32()?;

        let meta_data_count = file.read_u32()?;
        header.size = file.read_u32()? + 0x40;
        for _ in 0..meta_data_count {
            let mut entry = MetaData::default();

            entry.unk0 = file.read_u32()?;
            entry.unk1 = file.read_u128()?;
            let string_len = file.read_u32()?;
            entry.name_data = file.read_vector(string_len as usize)?;

            header.meta_data.push(entry);
        }
        
        debug_assert!(!NODE_CLASSES.contains(&&header.class[..]) || (meta_data_count == 0 && header.size == 0x40));
        debug_assert_eq!(file.position()?, header.base + header.size as u64);

        Ok(Some(header))
    }

    /// Writes resource header to the file, will panic if the class is wrong
    pub fn write<T: BinaryStream>(&self, file : &mut T) -> io::Result<()> {
        const PAD : [u8; 0x20] = [0u8; 0x20];

        debug_assert!(self.class.len() < 0x20);
        file.write_u32(RESOURCE_MAGIC)?;

        let class = self.class.as_bytes();
        debug_assert!(self.class.len() == class.len()); // it better be ASCII
        file.write_bytes(class)?;
        file.write_bytes(&PAD[class.len()..])?;
        
        file.write_u128(self.uuid)?;
        file.write_u32(self.unk0)?;

        file.write_u32(self.meta_data.len() as u32)?;

        let mut meta_data_size = 0;
        for entry in &self.meta_data {
            meta_data_size += 4 + 16 + 4 + entry.name_data.len();
        }

        file.write_u32(meta_data_size as u32)?;
        for entry in &self.meta_data {
            file.write_u32(entry.unk0)?;
            file.write_u128(entry.unk1)?;
            file.write_bytes(&entry.name_data[..])?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Getters)]
pub struct ResourceNode {
    /* Public info */

    #[getset(get = "pub")]
    header: ResourceHeader,

    #[getset(get = "pub")]
    children: Vec<ResourceChild>,

    /* Implementation detail */

    state: NodeModifiedState,
    data_offset: u64,
    header_len: u64,
    data_len: u64,
}

fn check_stream_delimiter<T: BinaryStream>(file: &mut T) -> io::Result<()> {
    if file.read_u8()? != 0x01 {
        Err(io::Error::new(io::ErrorKind::InvalidData, "Bad stream delimiter"))
    } else {
        Ok(())
    }
}

fn write_stream_delimiter<T: BinaryStream>(file: &mut T) -> io::Result<()> {
    file.write_u8(0x01)
}

fn copy_child_data<T: BinaryStream>(input: &Rc<RefCell<UncompressedFile>>, output : &mut T, child: &ResourceChild) -> io::Result<()> {
    let mut input_file = input.borrow_mut();
    input_file.seek(child.node_base + child.offset)?;
    input_file.copy_data(output, child.size as usize)?;
    Ok(())
}

impl ResourceNode {
    pub fn children_mut(&mut self) -> &mut[ResourceChild] {
        &mut self.children[..]
    }

    fn update_state(&mut self) {
        // DEBUG!!
        self.state = NodeModifiedState::Full;
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

    fn set_stream_ref(&mut self, stream: Weak<RefCell<UncompressedFile>>) {
        for child in &mut self.children {
            child.input_file = stream.clone();
            if let ResourceType::Node(node) = &mut child.contents {
                node.set_stream_ref(stream.clone());
            }
        }
    }

    fn write<T: BinaryStream>(&self, interface : &PakInterface, file : &mut T) -> io::Result<u64> {
        let offset_start = file.position()?;
        let child_count = self.children.len();

        // write basic header
        self.header.write(file)?;
        write_stream_delimiter(file)?;
        file.write_u32(0x1000000)?;
        file.write_u32(child_count as u32)?;
        file.write_u32(4)?;
        write_stream_delimiter(file)?;

        // write file list

        for child in &self.children {
            file.write_string(&child.name)?;
        }

         write_stream_delimiter(file)?;

        let offset_section = file.position()?;

        // nasty but the easiest way to do this in rust
        let offset_data_temp = vec![0u8; child_count * 8 + 1 + child_count * 4];
        file.write_bytes(&offset_data_temp)?;

        write_stream_delimiter(file)?;

        for child in &self.children {
            match child.contents {
                ResourceType::Link(_) => { file.write_u32(1) }
                _=> { file.write_u32(0) }
            }?
        }

        #[derive(Debug, Default, Clone, Copy)]
        struct ChildInfo {
            size: u32,
            offset: u64
        }
        let mut new_info = vec![ChildInfo::default(); child_count];
        // copy old offset and size info
        for child_index in 0..child_count {
            new_info[child_index].offset = self.children[child_index].offset;
            new_info[child_index].size = self.children[child_index].size;
        }

        if self.state == NodeModifiedState::Clean {
            let mut input_file = interface.input_file.borrow_mut();
            input_file.seek(self.data_offset)?;
            input_file.copy_data(file, self.data_len as usize)?;
        } else { // todo use the other state info or remove it

            // write child data
            for child_index in 0..child_count {
                let child = &self.children[child_index];
                new_info[child_index].offset = file.position()? - offset_start;
                match &child.contents {
                    ResourceType::Data => {
                        match &child.new_data {
                            Some(data) => { file.write_bytes(data)?; }
                            None => { copy_child_data(&interface.input_file, file, child)?; }
                        }
                    },
                    ResourceType::Resource(header) => {
                        match &child.new_data {
                            Some(data) => { 
                                header.write(file)?;
                                file.write_bytes(data)?; 
                            }
                            None => { copy_child_data(&interface.input_file, file, child)?; }
                        }
                    },
                    ResourceType::Node(node) => {
                        if node.state != NodeModifiedState::Clean {
                            new_info[child_index].size = node.write(interface, file)? as u32;
                        } else {
                            copy_child_data(&interface.input_file, file, child)?;
                        }
                    }
                    _=> continue
                }
            }

            // update links
            for child_index in 0..child_count {
                if let ResourceType::Link(idx) = self.children[child_index].contents {
                    new_info[child_index].offset = new_info[idx].offset;
                }
            }
        }

        let end = file.position()?;

        file.seek(offset_section)?;

        for info in &new_info {
            file.write_u64(info.offset)?;
        }

        write_stream_delimiter(file)?;

        for info in &new_info {
            file.write_u32(info.size)?;
        }

        file.seek(end)?;
        Ok(end - offset_start)
    }
    
    fn read<T: BinaryStream>(file: &mut T, header: ResourceHeader) ->io::Result<Self> {
        // children are relative to node base
        let node_base = file.position()? - header.size as u64;

        let mut node = Self::default();

        node.header = header;

        check_stream_delimiter(file)?;

        assert_eq!(file.read_u32()?, 0x1000000);
        let child_count = file.read_u32()?;
        assert_eq!(file.read_u32()?, 4);

        check_stream_delimiter(file)?;

        for _ in 0..child_count {
            let mut child = ResourceChild::default();
            child.node_base = node_base;
            child.name = file.read_string()?;

            node.children.push(child);
        }
        check_stream_delimiter(file)?;

        for child_index in 0..child_count as usize {
            node.children[child_index].offset = file.read_u64()?;
        }

        check_stream_delimiter(file)?;

        for child_index in 0..child_count as usize {
            node.children[child_index].size = file.read_u32()?;
        }

        check_stream_delimiter(file)?;
        let mut is_link = vec![false; child_count as usize];
        for child_index in 0..child_count as usize {
            is_link[child_index] = file.read_u32()? > 0;
        }

        node.data_offset = file.position()?;
        node.header_len = node.data_offset - node_base;

        for child_index in 0..child_count as usize {
            if !is_link[child_index] {
                let node_file_offset = node_base + node.children[child_index].offset;
                file.seek(node_file_offset)?;
                if let Some(header) = ResourceHeader::read(file)? {
                    if NODE_CLASSES.contains(&&header.class[..]) {
                        let mut child_node = Self::read(file, header)?;
                        child_node.data_len = node.children[child_index].size as u64 - (child_node.data_offset - node_file_offset);
                        node.children[child_index].contents = ResourceType::Node(child_node);
                    } else {
                        node.children[child_index].contents = ResourceType::Resource(header);
                    }
                }
            } else {
                for other_child_index in 0..child_count as usize {
                    if !is_link[other_child_index] &&
                            node.children[other_child_index].offset == node.children[child_index].offset {
                        node.children[child_index].contents = ResourceType::Link(other_child_index);
                        node.children[other_child_index].has_active_links = true; // prevent it from being moved or deleted
                        break;
                    }
                }
            }
        }
        Ok(node)
    }
}

#[derive(Debug)]
pub struct PakInterface {
    input_file: std::rc::Rc<RefCell<UncompressedFile>>,
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

    pub fn open(file : File) -> io::Result<PakInterface> {
        let mut file = UncompressedFile::new(file);
        if let Some(header_data) = ResourceHeader::read(&mut file)? {
            let mut root_node = ResourceNode::read(&mut file, header_data)?;
            root_node.data_len = file.length()? - root_node.data_offset;
            let stream_ref = Rc::new(RefCell::new(file));
            root_node.set_stream_ref(Rc::downgrade(&stream_ref));
            Ok(PakInterface { input_file: stream_ref, root_node: root_node })
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "Bad header"))
        }
    }

    pub fn save(&mut self, file : File) -> io::Result<()> {
        let mut file = UncompressedFile::new(file);
        self.root_node.update_state();
        self.root_node.write(self, &mut file)?;
        Ok(())
    }
}
