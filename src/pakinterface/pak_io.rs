use std::{io::{self, Read, Seek, SeekFrom, Write}};
use crate::util::{BinaryStream};
#![feature(seek_stream_len)]

#[derive(Debug)]
pub struct UncompressedFile {
    file: std::fs::File
}

impl UncompressedFile {
    pub fn new(file: std::fs::File) -> Self {
        UncompressedFile { file : file }
    }
}

impl BinaryStream for UncompressedFile {
    fn read_bytes(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.file.read_exact(buf)
    }

    fn write_bytes(&mut self, buf: &[u8]) -> io::Result<()> {
        self.file.write_all(buf)
    }

    fn seek(&mut self, pos: u64) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(pos))?;
        Ok(())
    }

    fn position(&mut self) -> io::Result<u64> {
        self.file.stream_position()
    }

    fn length(&mut self) -> io::Result<u64> {
        self.file.stream_len()
    }
}
