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
    Ok(buffer[0])
}

// shamelessly taken from https://github.com/camden-smallwood/zeta/blob/master/blam/src/datatypes/static_array.rs
// thanks @camden-smallwood
use std::{cmp, fmt, ops::{Index, IndexMut, Range, RangeFull}};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StaticArray<T, const N: usize>([T; N]);

impl<T, const N: usize> StaticArray<T, N> {
    pub fn len(self) -> usize {
        N
    }
}

impl<T: Default + Copy, const N: usize> Default for StaticArray<T, N> {
    fn default() -> Self {
        Self ([Default::default(); {N}])
    }
}

impl<T, const N: usize> Index<usize> for StaticArray<T, N> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &T {
        &self.0[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for StaticArray<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.0[index]
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for StaticArray<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0[..].fmt(f)
    }
}

impl<T, const N: usize> Index<Range<usize>> for StaticArray<T, N> {
    type Output = [T];
    #[inline]
    fn index(&self, index: Range<usize>) -> &[T] {
        &self.0[index]
    }
}

impl<T, const N: usize> IndexMut<Range<usize>> for StaticArray<T, N> {
    #[inline]
    fn index_mut(&mut self, index: Range<usize>) -> &mut [T] {
        &mut self.0[index]
    }
}

impl<T, const N: usize> Index<RangeFull> for StaticArray<T, N> {
    type Output = [T];
    #[inline]
    fn index(&self, _index: RangeFull) -> &[T] {
        &self.0[..]
    }
}

impl<T, const N: usize> IndexMut<RangeFull> for StaticArray<T, N> {
    #[inline]
    fn index_mut(&mut self, _index: RangeFull) -> &mut [T] {
        &mut self.0[..]
    }
}

impl<T: PartialEq, const N: usize> PartialEq for StaticArray<T, N> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
    
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        self.0[..] != other.0[..]
    }
}

impl<T: PartialOrd, const N: usize> PartialOrd for StaticArray<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&&self.0[..], &&other.0[..])
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&&self.0[..], &&other.0[..])
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&&self.0[..], &&other.0[..])
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&&self.0[..], &&other.0[..])
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&&self.0[..], &&other.0[..])
    }
}
