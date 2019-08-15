/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */
use std::ops::{Index, IndexMut, Range};
use std::ptr::NonNull;
use bytes::BufMut;

use crate::results::MemError;
use crate::hypervec::{HyperVec, ReadVisitor, WriteVisitor, Endianness};
use std::sync::atomic::AtomicUsize;
use std::alloc::{Alloc, Layout};

use serde::{Serialize, Deserialize};

/// For serialization purposes
#[repr(C)]
#[derive(Serialize, Deserialize)]
pub struct HyperVecSerde(pub Vec<u8>, pub isize, pub usize, pub usize, pub bool);


impl HyperVecSerde {
    /// #
    pub fn serialize_to_disk(self, path: &str) -> Result<usize, std::io::Error> {
        crate::util::ser::serialize_hypervec_to_disk(path, &self)
    }

    /// #
    pub fn deserialize_from_disk(path: &str) -> Result<Self, std::io::Error> {
        crate::util::ser::deserialize_hypervec_from_disk(path)
    }
}

impl Into<HyperVecSerde> for HyperVec  {
    fn into(mut self) -> HyperVecSerde {
        let bytes =  unsafe { self.get_full_bytes_mut().to_vec()};
        let cursor = self.cursor;
        let read_version = self.get_read_version();
        let write_version = self.get_write_version();
        let is_be = self.endianness.is_be();
        HyperVecSerde (
            bytes,
            cursor,
            read_version,
            write_version,
            is_be
        )
    }
}

impl Into<HyperVec> for HyperVecSerde {
    fn into(self) -> HyperVec{
        let mut hvec = HyperVec::wrap_bytes(self.0);
        hvec.cursor = self.1;
        hvec.read_version = AtomicUsize::new(self.2);
        hvec.write_version = AtomicUsize::new(self.3);
        hvec.endianness = Endianness::from_bool(self.4);
        hvec
    }
}

/// The primary HyperVec is allowed to ship around between threads
unsafe impl Send for HyperVec {}

/// Data races can be fully prevented by using WriteVisitors; However, these devices cannot be shipped between threads, and should only be used for future writes
unsafe impl Sync for HyperVec{}

impl Drop for HyperVec {
    fn drop(&mut self) {
        unsafe { std::alloc::Global.dealloc(NonNull::new(self.ptr).unwrap(), self.layout) }
    }
}

/*
#[allow(ambiguous_associated_items)]
impl<'visit, T, E: AsRef<[u8]>> Try for MemoryResult<WriteVisitor<'visit, T>, MemError<'visit, E>> {
    type Ok = WriteVisitor<'visit, T>;
    type Error = MemError<'visit, E>;

    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        self
    }

    fn from_error(err: Self::Error) -> Self {
        panic!("Invalid operation. Error: {}", err.to_string())
    }

    fn from_ok(v: Self::Ok) -> Self {
        v
    }
}
*/
impl Index<isize> for HyperVec {
    type Output = u8;

    fn index(&self, index: isize) -> &Self::Output {
        unsafe { &*self.ptr.offset(index) }
    }
}

impl Index<Range<isize>> for HyperVec {
    type Output = [u8];

    #[inline]
    fn index(&self, index: Range<isize>) -> &Self::Output {
        unsafe { &*std::ptr::slice_from_raw_parts(&*self.ptr.offset(index.start), (index.end - index.start) as usize) }
    }
}

impl IndexMut<isize> for HyperVec {
    #[inline]
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        unsafe { &mut *self.ptr.offset(index) }
    }
}

impl IndexMut<Range<isize>> for HyperVec {
    fn index_mut(&mut self, index: Range<isize>) -> &mut Self::Output {
        unsafe { &mut *std::ptr::slice_from_raw_parts_mut(&mut *self.ptr.offset(index.start), (index.end - index.start) as usize) }
    }
}

impl Iterator for HyperVec {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.len as isize {
            let ret = self[self.cursor];
            self.cursor += 1;
            Some(ret)
        } else {
            None
        }
    }
}

/// Unlike BufExtend, this necessariy requires the capacity to accomidate the placed bytes
impl BufMut for HyperVec {
    fn remaining_mut(&self) -> usize {
        self.len - (self.cursor as usize)
    }
    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.advance_cursor_by(cnt);
    }
    unsafe fn bytes_mut(&mut self) -> &mut [u8] {
        self.get_bytes_mut_cursor()
    }

    #[inline]
    #[expand(depth = 5, expression = "*self.ptr.offset(p0 + {}) = slice[{}]")]
    fn put_slice(&mut self, slice: &[u8]) {
        unsafe {
            debug_assert!(self.remaining_mut() >= slice.len());
            let p0 = self.cursor;
            println!("putting w/ cursor pos {}", p0);
            let len = slice.len() as isize;
            match len {}
            self.cursor += len;
        }
    }

    #[inline]
    fn put_u8(&mut self, val: u8) {
        self.put_slice(&[val]);
    }
}

/// Used to cast the internal of a HyperVec
pub trait Castable {
    /// Casts the underlying bytes to an immutable version of the the supplied type with checking alignment accompanied by a ReadVisitor
    fn cast<Type: ?Sized>(&self) -> Result<ReadVisitor<Type>, MemError<&[u8]>>;
    /// Casts the underlying bytes to an immutable version of the the supplied type without checking alignment
    unsafe fn cast_unchecked<Type: ?Sized>(&self) -> &Type;
    /// Casts the underlying type to an array of the user-specified type. If the user is referencing an array of u16's, then when
    /// cast or cast_mut is called prior to this (to obtain the appropriate visitor), then the type paremeter should be "u16", but
    /// NOT "[u16]"
    unsafe fn cast_unchecked_array<Type: Sized>(&self) -> &[Type];
    /// Casts the underlying bytes to a mutable version of the supplied type with checking alignment accompanied by a WriteVisitor
    fn cast_mut<Type: ?Sized>(&mut self) -> Result<WriteVisitor<Type>, MemError<&[u8]>>;
    /// Casts the underlying bytes to a mutable version of the the supplied type without checking alignment
    unsafe fn cast_unchecked_mut<Type: ?Sized>(&mut self) -> &mut Type;

    /// Casts the underlying type to an array of the user-specified type. If the user is referencing an array of u16's, then when
    /// cast or cast_mut is called prior to this (to obtain the appropriate visitor), then the type paremeter should be "u16", but
    /// NOT [u16]
    unsafe fn cast_unchecked_mut_array<Type: Sized>(&mut self) -> &mut [Type];
}


impl Castable for HyperVec {
    fn cast<Type: ?Sized>(&self) -> Result<ReadVisitor<Type>, MemError<&[u8]>> {
        //println!("{} {} | {} {}", std::mem::align_of::<&Type>(), self.layout.align(), std::mem::size_of::<&Type>(), self.layout.size());
        Ok(ReadVisitor::new(self as *const Self as *mut Self, self.get_write_version()))
    }

    unsafe fn cast_unchecked<Type: ?Sized>(&self) -> &Type {
        //println!("S/A {} / {}", std::mem::size_of::<&Type>(), std::mem::align_of::<&Type>());
        std::mem::transmute_copy::<*mut u8, &mut Type>(&self.ptr)
    }

    unsafe fn cast_unchecked_array<Type: Sized>(&self) -> &[Type] {
        //println!("S/A {} / {}", std::mem::size_of::<Type>(), std::mem::align_of::<Type>());
        let base_ptr = std::mem::transmute_copy::<*const u8, *const Type>(&(self.ptr as *const u8));
        &*std::ptr::slice_from_raw_parts(base_ptr, self.len / std::mem::size_of::<Type>())
    }

    fn cast_mut<Type: ?Sized>(&mut self) -> Result<WriteVisitor<Type>, MemError<&[u8]>> {
        //println!("{} {} | {} {}", std::mem::align_of::<&Type>(), self.layout.align(), std::mem::size_of::<&Type>(), self.layout.size());
        Ok(WriteVisitor::new(&mut *self as *mut Self, self.get_write_version()))
    }

    unsafe fn cast_unchecked_mut<Type: ?Sized>(&mut self) -> &mut Type {
        //println!("S/A {} / {}", std::mem::size_of::<&Type>(), std::mem::align_of::<&Type>());
        std::mem::transmute_copy::<*mut u8, &mut Type>(&self.ptr)
    }

    unsafe fn cast_unchecked_mut_array<Type: Sized>(&mut self) -> &mut [Type] {
        //println!("S/A {} / {}", std::mem::size_of::<Type>(), std::mem::align_of::<Type>());
        let base_ptr = std::mem::transmute_copy::<*mut u8, *mut Type>(&self.ptr);
        &mut *std::ptr::slice_from_raw_parts_mut(base_ptr, self.len / std::mem::size_of::<Type>())
    }
}


/// Byte-order aware wrapper for data allocation within a [HyperVec]
pub trait ByteWrapper {
    /// Returns a byte-wrapped HyperVec
    fn wrap_bytes<T: AsRef<[u8]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u16s decomposed into u8s
    fn wrap_u16s_be<T: AsRef<[u16]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u32s decomposed into u8s
    fn wrap_u32s_be<T: AsRef<[u32]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u64s decomposed into u8s
    fn wrap_u64s_be<T: AsRef<[u64]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u128s decomposed into u8s
    fn wrap_u128s_be<T: AsRef<[u128]>>(t: T) -> HyperVec;

    /// Returns a LittleEndian ordered HyperVec of i32s decomposed into u8s
    fn wrap_u16s_le<T: AsRef<[u16]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u32s decomposed into u8s
    fn wrap_u32s_le<T: AsRef<[u32]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u64s decomposed into u8s
    fn wrap_u64s_le<T: AsRef<[u64]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u128s decomposed into u8s
    fn wrap_u128s_le<T: AsRef<[u128]>>(t: T) -> HyperVec;


    /// Returns a BigEndian ordered HyperVec of i32s decomposed into u8s
    fn wrap_i16s_be<T: AsRef<[i16]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u32s decomposed into u8s
    fn wrap_i32s_be<T: AsRef<[i32]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u64s decomposed into u8s
    fn wrap_i64s_be<T: AsRef<[i64]>>(t: T) -> HyperVec;
    /// Returns a BigEndian ordered HyperVec of u128s decomposed into u8s
    fn wrap_i128s_be<T: AsRef<[i128]>>(t: T) -> HyperVec;

    /// Returns a LittleEndian ordered HyperVec of i32s decomposed into u8s
    fn wrap_i16s_le<T: AsRef<[i16]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u32s decomposed into u8s
    fn wrap_i32s_le<T: AsRef<[i32]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u64s decomposed into u8s
    fn wrap_i64s_le<T: AsRef<[i64]>>(t: T) -> HyperVec;
    /// Returns a LittleEndian ordered HyperVec of u128s decomposed into u8s
    fn wrap_i128s_le<T: AsRef<[i128]>>(t: T) -> HyperVec;
}

impl ByteWrapper for HyperVec {
    #[inline]
    /// Wraps around a pre-existing item that can be represented by a vecotr of its components, then translates them into bytes
    fn wrap_bytes<T: AsRef<[u8]>>(t: T) -> Self {
        let t = t.as_ref();
        let len = t.len();
        let layout = Layout::array::<u8>(len).unwrap();
        println!("Align: {}, Size: {}", std::mem::align_of_val(&t), std::mem::size_of_val(&t));

        let ptr0 = (&*t as *const [u8]) as *const u8;

        let ptr = unsafe { std::alloc::alloc(layout) };
        unsafe { std::ptr::copy_nonoverlapping(ptr0, ptr, layout.size()) };

        Self {ptr, len, cursor: 0, read_version: AtomicUsize::new(0), write_version: AtomicUsize::new(0), corrupt: false, layout, endianness: Endianness::target(), partition_map: None}
    }

    /// Wraps around an array of u16's, and returns a vector comprised of fundamental u8 bytes
    fn wrap_u16s_be<T: AsRef<[u16]>>(t: T) -> Self {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u16>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u32s_be<T: AsRef<[u32]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u32>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u64s_be<T: AsRef<[u64]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u64>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u128s_be<T: AsRef<[u128]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u128>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u16s_le<T: AsRef<[u16]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u16>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u32s_le<T: AsRef<[u32]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u32>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u64s_le<T: AsRef<[u64]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u64>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_u128s_le<T: AsRef<[u128]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<u128>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i16s_be<T: AsRef<[i16]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i16>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i32s_be<T: AsRef<[i32]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i32>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i64s_be<T: AsRef<[i64]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i64>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i128s_be<T: AsRef<[i128]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i128>());
        for i in t {
            res.extend(i.to_be_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i16s_le<T: AsRef<[i16]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i16>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i32s_le<T: AsRef<[i32]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i32>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i64s_le<T: AsRef<[i64]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i64>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }

    fn wrap_i128s_le<T: AsRef<[i128]>>(t: T) -> HyperVec {
        let t = t.as_ref();
        let mut res = Vec::with_capacity(t.len() * std::mem::size_of::<i128>());
        for i in t {
            res.extend(i.to_le_bytes().iter().copied());
        }
        Self::wrap_bytes(res)
    }
}

/// Unlike BufMut, BytePusher's impl's will resize HyperVec's buffer before pushing the bytes inwards
/// WARNING: This does not do any Bounds' checking! Make sure that the space does indeed require to be pushed.
/// This will load data at the cursor position; put_u8 (or putter functions in general) are what ought to be used
/// when the vector is pre-allocated
pub trait BytePusher {
    /// Extends the underlying buffer by len * size_of::<u8>(), and pushes each byte thereon (this is the only place where extension occurs; the others helper subroutines simply push converted bytes into here)
    fn push_u8s<T: AsRef<[u8]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<u16>(), Converts each u16 into the internal buffer's set order, then pushes each byte into the buffer
    fn push_u16s<T: AsRef<[u16]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<u32>(), Converts each u32 into the internal buffer's set order, then pushes each byte into the buffer
    fn push_u32s<T: AsRef<[u32]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<u64>(), Converts each u64 into the internal buffer's set order, then pushes each byte into the buffer
    fn push_u64s<T: AsRef<[u64]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<u128>(), Converts each u128 into the internal buffer's set order, then pushes each byte into the buffer
    fn push_u128s<T: AsRef<[u128]>>(&mut self, t: T);

    /// Extends the underlying buffer by len * size_of::<i8>(), converts each i8 into a u8, and pushes each byte into the buffer with its internally-specified byte order
    fn push_i8s<T: AsRef<[i8]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<i16>(), converts each i16 into a u16, and pushes each byte into the buffer with its internally-specified byte order
    fn push_i16s<T: AsRef<[i16]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<i32>(), converts each i32 into a u32, and pushes each byte into the buffer with its internally-specified byte order
    fn push_i32s<T: AsRef<[i32]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<i64>(), converts each i64 into a u64, and pushes each byte into the buffer with its internally-specified byte order
    fn push_i64s<T: AsRef<[i64]>>(&mut self, t: T);
    /// Extends the underlying buffer by len * size_of::<i128>(), converts each i128 into a u128, and pushes each byte into the buffer with its internally-specified byte order
    fn push_i128s<T: AsRef<[i128]>>(&mut self, t: T);
}


/// TODO: Streamline the code below for less repetition
impl BytePusher for HyperVec {
    #[inline]
    fn push_u8s<T: AsRef<[u8]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len());
        self.put_slice(t);
    }

    fn push_u16s<T: AsRef<[u16]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 2);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_u32s<T: AsRef<[u32]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 4);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_u64s<T: AsRef<[u64]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 8);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_u128s<T: AsRef<[u128]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 16);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_i8s<T: AsRef<[i8]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len());

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_i16s<T: AsRef<[i16]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 2);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_i32s<T: AsRef<[i32]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 4);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_i64s<T: AsRef<[i64]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 8);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }

    fn push_i128s<T: AsRef<[i128]>>(&mut self, t: T) {
        let t  = t.as_ref();
        self.extend(t.len() * 16);

        match self.endianness {
            Endianness::BE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_be_bytes().as_ref())
                })
            },

            Endianness::LE => {
                t.iter().for_each(|val| {
                    self.put_slice(val.to_le_bytes().as_ref())
                })
            }
        }
    }
}