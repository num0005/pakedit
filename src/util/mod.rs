use std::{io};

pub const fn u32_from_str(input: &'static str) -> u32 {
    let bytes = input.as_bytes();
    (bytes[0] as u32) << 24 |
    (bytes[1] as u32) << 16 |
    (bytes[2] as u32) << 8  |
    (bytes[3] as u32) << 0
}

pub trait BinaryStream {
    /// Read buffer at current offset
    fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<()>;

    fn read_vector(&mut self, count: usize) -> io::Result<Vec<u8>> {
        let mut data = vec![0u8; count];
        self.read_bytes(&mut data[..])?;
        Ok(data)
    } 

    /// Write buffer at current offset
    fn write_bytes(&mut self, buf: &[u8]) -> io::Result<()>;

    /// Attempt to seek to offset
    fn seek(&mut self, pos: u64) -> io::Result<()>;

    /// Returns current offset
    fn position(&mut self) -> io::Result<u64>;

    // Returns stream length
    fn length(&mut self) -> io::Result<u64>;

    /* Basic reader functions */

    fn read_u128(&mut self) -> io::Result<u128> {
        let mut buffer: [u8; 16] = Default::default();
        self.read_bytes(&mut buffer)?;
        Ok(u128::from_le_bytes(buffer))
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buffer: [u8; 8] = Default::default();
        self.read_bytes(&mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buffer: [u8; 4] = Default::default();
        self.read_bytes(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        let mut buffer: [u8; 2] = Default::default();
        self.read_bytes(&mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buffer: [u8; 1] = Default::default();
        self.read_bytes(&mut buffer)?;
        Ok(u8::from_le_bytes(buffer))
    }

    /// Read a dynamic length string starting with the length as a u32
    fn read_string(&mut self) -> io::Result<String> {
        let string_len = self.read_u32()? as usize;
    
        let mut data = vec![0u8; string_len];
        self.read_bytes(&mut data)?;
    
        if let Ok(string) = String::from_utf8(data) {
            Ok(string)
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "Bad string"))
        }
    }

    /* Basic writter functions */

    fn write_u128(&mut self, value: u128) -> io::Result<()> {
        let buffer = u128::to_le_bytes(value);
        self.write_bytes(&buffer)
    }

    fn write_u64(&mut self, value: u64) -> io::Result<()> {
        let buffer = u64::to_le_bytes(value);
        self.write_bytes(&buffer)
    }

    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        let buffer = u32::to_le_bytes(value);
        self.write_bytes(&buffer)
    }

    fn write_u16(&mut self, value: u16) -> io::Result<()> {
        let buffer = u16::to_le_bytes(value);
        self.write_bytes(&buffer)
    }

    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        let buffer = u8::to_le_bytes(value);
        self.write_bytes(&buffer)
    }

    /// Write a dynamic length string starting with the length as a u32
    fn write_string(&mut self, value: &String) -> io::Result<()> {
        let data = value.as_bytes();
        self.write_u32(data.len() as u32)?;
        self.write_bytes(data)
    }

    /// Copies data from this stream to another
    fn copy_data<T : BinaryStream>(&mut self, output_file : &mut T, size : usize) -> io::Result<()> {
        const BUFFER_SIZE: usize = 0x40000;
        let mut bytes_left = size;
        while bytes_left > 0 {
            let bytes_to_read = std::cmp::min(bytes_left, BUFFER_SIZE);
            let mut buffer = [0u8; BUFFER_SIZE];
            self.read_bytes(&mut buffer[..bytes_to_read])?;
            output_file.write_bytes(&buffer[..bytes_to_read])?;
            bytes_left -= bytes_to_read;
        }
        Ok(())
    }
}

pub fn string_length(data: &[u8]) -> usize {
    for offset in 0..data.len() {
        if data[offset] == 0 {
            return offset
        }
    }
    data.len()
}
