use std::{fs::File, io::{self, Read, Write}};

pub const fn u32_from_str(input: &'static str) -> u32 {
    let bytes = input.as_bytes();
    (bytes[0] as u32) << 24 |
    (bytes[1] as u32) << 16 |
    (bytes[2] as u32) << 8 |
    (bytes[3] as u32) << 0
}

pub fn read_u128(mut file: &File) -> io::Result<u128> {
    let mut buffer: [u8; 16] = Default::default();
    file.read(&mut buffer)?;
    Ok(u128::from_le_bytes(buffer))
}

pub fn read_u64(mut file: &File) -> io::Result<u64> {
    let mut buffer: [u8; 8] = Default::default();
    file.read(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

pub fn read_u32(mut file: &File) -> io::Result<u32> {
    let mut buffer: [u8; 4] = Default::default();
    file.read(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

pub fn read_u16(mut file: &File) -> io::Result<u16> {
    let mut buffer: [u8; 2] = Default::default();
    file.read(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

pub fn read_u8(mut file: &File) -> io::Result<u8> {
    let mut buffer: [u8; 1] = Default::default();
    file.read(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

pub fn write_u128(mut file: &File, value: u128) -> io::Result<()> {
    let buffer = u128::to_le_bytes(value);
    file.write_all(&buffer)
}

pub fn write_u64(mut file: &File, value: u64) -> io::Result<()> {
    let buffer = u64::to_le_bytes(value);
    file.write_all(&buffer)
}

pub fn write_u32(mut file: &File, value: u32) -> io::Result<()> {
    let buffer = u32::to_le_bytes(value);
    file.write_all(&buffer)
}

pub fn write_u16(mut file: &File, value: u16) -> io::Result<()> {
    let buffer = u16::to_le_bytes(value);
    file.write_all(&buffer)
}

pub fn write_u8(mut file: &File, value: u8) -> io::Result<()> {
    let buffer = u8::to_le_bytes(value);
    file.write_all(&buffer)
}

pub fn copy_data(mut input_file : &File, mut output_file : &File, size : usize) -> io::Result<()> {
    const BUFFER_SIZE: usize = 0x40000;
    let mut bytes_left = size;
    while bytes_left > 0 {
        let bytes_to_read = std::cmp::min(bytes_left, BUFFER_SIZE);
        let mut buffer = [0u8; BUFFER_SIZE];
        input_file.read_exact(&mut buffer[..bytes_to_read])?;
        output_file.write_all(&buffer[..bytes_to_read])?;
        bytes_left -= bytes_to_read;
    }
    Ok(())
}
