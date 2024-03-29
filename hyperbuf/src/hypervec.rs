/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution. 
 */

use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use bytes::BufMut;
use crate::results::{InformationResult, MemError};
use crate::impls::*;
use crate::partition_map::PartitionMap;
use std::fmt::{Display, Formatter, Error};

/// This is a type which can be re-interpreted to any type, regardless of alignment
#[fundamental]
#[repr(C)]
pub struct HyperVec {
    /// #
    pub ptr: *mut u8,
    pub(crate) len: usize,
    pub(crate) cursor: isize,
    /// The read and write versions are only for editing data through visitors
    pub(crate) read_version: AtomicUsize,
    pub(crate) write_version: AtomicUsize,
    /// See [WriteVisitor] for the definition of "corrupt"
    pub(crate) corrupt: bool,
    pub(crate) endianness: Endianness,
    pub(crate) partition_map: Option<PartitionMap>,
    /// We place the layout at the end of the struct to ensure that, in the event of corruption, the bytes do not interfere with this struct.
    pub(crate) layout: Layout
}

impl HyperVec {
    #[inline]
    /// Returns a HyperVec module that is blocked
    pub fn new(len: usize) -> Self {
        let layout = Layout::array::<u8>(len).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        Self { ptr, len, layout, cursor: 0, read_version: AtomicUsize::new(0), write_version: AtomicUsize::new(0), corrupt: false, endianness: Endianness::target(), partition_map: None}
    }

    #[inline]
    /// Returns a HyperVec module that is blocked
    pub fn new_zeroed(len: usize) -> Self {
        let layout = Layout::array::<u8>(len).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        Self { ptr, len, layout, cursor: 0, read_version: AtomicUsize::new(0), write_version: AtomicUsize::new(0), corrupt: false, endianness: Endianness::target(), partition_map: None}
    }

    #[inline]
    /// Wraps around a pre-existing value, translating it into its bytes.
    /// Use wrap_bytes for arrays; this is more for structs
    pub fn wrap<T: ?Sized>(t: &T) -> Self {
        let ptr0 = t as *const T as *const u8;
        println!("[WRAP] {} {}", std::mem::size_of_val(t), std::mem::align_of_val(t));
        let layout = Layout::for_value::<T>(t);
        let ptr = unsafe { std::alloc::alloc(layout) };

        println!("LAYOUT size: {}", layout.size());

        unsafe { std::ptr::copy_nonoverlapping(ptr0, ptr, layout.size()) };

        Self {
            ptr,
            len: layout.size(),
            cursor: 0,
            read_version: AtomicUsize::new(0),
            write_version: AtomicUsize::new(0),
            corrupt: false,
            layout,
            endianness: Endianness::target(),
            partition_map: None
        }
    }

    /// Debug ONLY
    #[allow(dead_code)]
    pub fn as_static(&mut self) -> &'static mut Self {
        unsafe { std::mem::transmute::<&mut Self, &'static mut Self>(self) }
    }

    /// Saves the data the the disk, and returns the number of bytes written if successful
    /// NOT WORKING
    pub fn serialize_to_disk(self, path: &str) -> Result<usize, std::io::Error> {
        if self.is_corrupted() {
            return MemError::throw_std("You cannot serialize a corrupted dataset; this is to ensure the data you want is going to be written, and not junk data");
        }

        let res: HyperVecSerde = self.into();
        res.serialize_to_disk(path)
    }

    /// Retrieves a HyperVec from the disk
    /// NOT WORKING
    pub async fn deserialize_from_disk(path: &str) -> Result<HyperVec, std::io::Error> {
        HyperVecSerde::deserialize_from_disk(path)
            .and_then(|raw| {
                Ok(raw.into())
        })
    }

    /// Returns the number of bytes
    pub fn length(&self) -> usize {
        self.len
    }

    /// Return an immutable slice of the underlying bytes
    pub unsafe fn bytes(&self) -> &[u8] {
        &*std::ptr::slice_from_raw_parts(self.ptr, self.len)
    }

    /// Return an mutable slice of the underlying bytes
    pub unsafe fn get_full_bytes_mut(&mut self) -> &mut [u8] {
        &mut *std::ptr::slice_from_raw_parts_mut(self.ptr, self.len)
    }

    /// Returns the bytes between the cursor position and the remaining mutable bytes on the heap
    pub unsafe fn get_bytes_mut_cursor(&mut self) -> &mut [u8] {
        &mut *std::ptr::slice_from_raw_parts_mut(self.ptr.offset(self.cursor), self.remaining_mut())
    }

    /// Returns the bytes between the cursor position and the remaining mutable bytes on the heap
    pub unsafe fn get_bytes_cursor(&mut self) -> &[u8] {
        &*std::ptr::slice_from_raw_parts(self.ptr.offset(self.cursor), self.remaining_mut())
    }

    /// Reads the cursor position
    pub fn cursor_position(&self) -> isize {
        self.cursor
    }

    /// Reads the value at the current cursor
    pub fn read_cursor(&self) -> u8 {
        unsafe { *self.ptr.offset(self.cursor) }
    }

    /// Reads the value at the supplied index which is offset from the intiial pointer
    pub fn read_relative(&self, pos: isize) -> u8 {
        unsafe { *self.ptr.offset(pos) }
    }

    /// Reads the value at the supplied index which is offset from the cursor position
    pub fn read_cursor_offset(&self, pos: isize) -> u8 {
        unsafe { *self.ptr.offset(self.cursor + pos) }
    }

    /// Advance the cursor by 1
    pub fn advance_cursor_by(&mut self, amt: usize) {
        self.cursor += amt as isize
    }

    /// Advance the cursor by 1
    pub fn advance_cursor(&mut self) {
        self.advance_cursor_by(1)
    }

    /// Get and advance
    pub fn get_and_advance_cursor(&mut self) -> u8 {
        self.advance_cursor();
        self.read_cursor_offset(-1)
    }

    /// Sets the cursor's position relative to the initial pointer
    pub fn set_cursor_pos(&mut self, pos: isize) {
        self.cursor = pos
    }

    /// Resets the cursor
    pub fn reset_cursor(&mut self) {
        self.cursor = 0;
    }

    #[inline]
    /// Relaxedly returns the write version
    pub fn get_write_version(&self) -> usize {
        self.write_version.load(Ordering::Relaxed)
    }

    #[inline]
    /// Relaxedly returns the read version
    pub fn get_read_version(&self) -> usize {
        self.read_version.load(Ordering::Relaxed)
    }

    /// This is safe since the operation is inherently atomic
    #[inline]
    pub unsafe fn get_and_increment_read_version(&self) -> usize {
        (*self).read_version.fetch_add(1, Ordering::SeqCst)
    }

    /// This is safe since the operation is inherently atomic
    #[inline]
    pub unsafe fn get_and_increment_write_version(&self) -> usize {
        (*self).write_version.fetch_add(1, Ordering::SeqCst)
    }

    /// This should only be called when no Read/WriteVisitors are active, otherwise setting this to another value will cause errors
    pub unsafe fn set_write_version(&mut self, update: usize) {
        self.write_version.store(update, Ordering::SeqCst);
    }

    /// This should only be called when no Read/WriteVisitors are active, otherwise setting this to another value will cause errors
    pub unsafe fn set_read_version(&mut self, update: usize) {
        self.read_version.store(update, Ordering::SeqCst);
    }

    /// Returns the buffer's endianness
    pub fn get_endianness(&self) -> &Endianness {
        &self.endianness
    }

    /// I am marking this function as unsafe, because if any downstream consumers depend upon the state of the bytes, then those consumers
    /// will possibly require to update the way they consume their data (if switched). This is to give the API programmer an idea of of the
    /// severity of this function
    pub unsafe fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    /// As writing occurs to the underlying object, it becomes entirely possible for the user to improperly use
    /// the WriteVisitor, thus signalling data corruption
    pub fn is_corrupted(&self) -> bool {
        self.corrupt
    }

    /// Extends the layout and increases the length
    #[allow(unused)]
    #[inline]
    pub fn extend(&mut self, additional_bytes: usize) {
        if let Ok((layout, pos_new)) = self.layout.extend(Layout::array::<u8>(additional_bytes).unwrap()) {
            println!("[REALLOC] additional bytes: {}", additional_bytes);
            println!("[REALLOC] new layout size, pos: {}, {}, --- {}", layout.size(), self.layout.size(), pos_new);
            assert_eq!(self.layout.size(), pos_new);
            println!("[REALLOC] self.len (before) {}", self.len);
            assert_eq!(layout.size(), additional_bytes + self.len);
            self.len += additional_bytes;
            println!("[REALLOC] self.len (after) {}", self.len);
            self.ptr = unsafe { std::alloc::realloc(self.ptr, layout, self.len) };
            self.layout = layout;
            println!("[REALLOC] {}", self);
        }
    }
}

/// Allows asynchronous data execution once it's spot in line reaches the 'front'.
pub struct WriteVisitor<'visit, T: ?Sized> {
    ptr: *mut HyperVec,
    ticket_number: usize,
    bytes_written: usize,
    _phantom: PhantomData<&'visit T>,
}

impl<'visit, T: ?Sized> !Send for WriteVisitor<'visit, T> {}

impl<'visit, T: ?Sized> !Sync for WriteVisitor<'visit, T> {}

#[allow(unused_results)]
impl<'visit, T: ?Sized> Drop for WriteVisitor<'visit, T> {
    fn drop(&mut self) {
        unsafe {
            //println!("DROPPING tx Ticket {}", self.ticket_number);
            let hvec = &mut *self.ptr;
            if self.bytes_written != 0 {
                hvec.extend(self.bytes_written);
            }
            hvec.get_and_increment_write_version();
        }
    }
}

impl<'visit, T: ?Sized> WriteVisitor<'visit, T> {
    /// Creates a new WriteVisitor
    pub fn new(hvec_ptr: *mut HyperVec, ticket_number: usize) -> Self {
        Self { ptr: hvec_ptr, ticket_number, _phantom: PhantomData, bytes_written: 0 }
    }

    /// Consumes the visitor. Make sure to enter at least the number of bytes you expect to extend into the buf in `pre_alloc` (if the current len does not suffice).
    /// The input subroutine must return the number of bytes written for verification.
    ///
    /// The input subroutine will be given a possibly existent mutable reference. The mutable reference may not exist if
    /// the item is "corrupted". The object T is defined as corrupt if the following occur
    ///
    /// [1] if the object was previously visited, but the returned subroutine's written amount was greater than the `pre_alloc`, then
    /// the bytes written to memory were corrupt. As such, the user should always manually check the return statement for a [MemError] type.
    ///
    /// [2]
    #[inline]
    pub async fn visit<Fx>(self, pre_alloc: Option<usize>, subroutine: Fx) -> Result<(), MemError<'visit, &'visit [u8]>> where Fx: Fn(&Self) -> Option<usize> {
        if let Some(alloc) = pre_alloc {
            unsafe { (*(self).ptr).extend(alloc) };
        }

        (&self).await.and_then(move |_| {
            self.visit_inner(pre_alloc, &subroutine)
        })
    }

    /// Quickly checks to see if the current writer is allowed to write, and if not, immediately returns with MemError::NOT_READY
    #[inline]
    pub unsafe fn try_visit<Fx>(self, pre_alloc: Option<usize>, subroutine: Fx) -> InformationResult<'visit, ()>
        where Fx: Fn(&Self) -> Option<usize> {
        if self.is_ready() {
            self.visit_inner(pre_alloc, &subroutine)
        } else {
            Err(MemError::NOT_READY)
        }
    }

    #[inline]
    fn visit_inner<Fx>(self, pre_alloc: Option<usize>, subroutine: &Fx) -> InformationResult<'visit, ()> where Fx: Fn(&Self) -> Option<usize> {
        unsafe {
            //println!("Will exec subroutine {}", self.ticket_number);
            let initial_size = (*(self).ptr).len;
            let pre_alloc_amt = pre_alloc.unwrap_or(0);

            match subroutine(&self) {
                Some(bytes_added) => {
                    if bytes_added > initial_size + pre_alloc_amt {
                        (*self.ptr).corrupt = true;
                        let bytes = (*self.ptr).bytes();
                        MemError::throw_corrupt(bytes)
                    } else {
                        Ok(())
                    }
                }

                _ => {
                    Ok(())
                }
            }
        }
    }

    #[inline]
    fn is_ready(&self) -> bool {
        unsafe {
            self.ticket_number == (*self.ptr).get_write_version()
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn write(&self) -> Option<&mut T> {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked_mut()) }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn write_array(&self) -> Option<&mut [T]> where for<'a> T: Sized + 'a {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked_mut_array()) }
        } else {
            None
        }
    }
}

impl<'visit, T: ?Sized> Future for & WriteVisitor<'visit, T> {
    type Output = InformationResult<'visit, ()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        if self.is_ready() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}


/// Allows asynchronous data execution once it's spot in line reaches the 'front'.
pub struct ReadVisitor<'visit, T: ?Sized> {
    ptr: *mut HyperVec,
    ticket_number: usize,
    bytes_written: usize,
    _phantom: PhantomData<&'visit T>,
}

impl<'visit, T: ?Sized> !Send for ReadVisitor<'visit, T> {}

impl<'visit, T: ?Sized> !Sync for ReadVisitor<'visit, T> {}

#[allow(unused_results)]
impl<'visit, T: ?Sized> Drop for ReadVisitor<'visit, T> {
    fn drop(&mut self) {
        unsafe {
            //println!("DROPPING rx Ticket {}", self.ticket_number);
            let hvec = &mut *self.ptr;
            if self.bytes_written != 0 {
                hvec.extend(self.bytes_written);
            }
            hvec.get_and_increment_read_version();
        }
    }
}

impl<'visit, T: ?Sized> ReadVisitor<'visit, T> {
    /// Creates a new WriteVisitor
    pub fn new(hvec_ptr: *mut HyperVec, ticket_number: usize) -> Self {
        Self { ptr: hvec_ptr, ticket_number, _phantom: PhantomData, bytes_written: 0 }
    }

    /// Consumes the visitor. Make sure to enter at least the number of bytes you expect to write in `pre_alloc` (if the current len does not suffice).
    /// The input subroutine must return the number of bytes written for verification.
    ///
    /// The input subroutine will be given a possibly existent mutable reference. The mutable reference may not exist if
    /// the item is "corrupted". The object T is defined as corrupt if the following occur
    ///
    /// [1] if the object was previously visited, but the returned subroutine's written amount was greater than the `pre_alloc`, then
    /// the bytes written to memory were corrupt. As such, the user should always manually check the return statement for a [MemError] type.
    ///
    /// [2] TBD
    #[allow(unused_must_use)]
    #[inline]
    async fn try_visit<Fx>(&self, subroutine: Fx) -> InformationResult<'visit, ()>
        where Fx: Fn(Option<&Self>) {
        // We need to check the write version to make sure it hasn't changed while waiting. IF a read
        // occurs simultaneous to a write, then that could mean that the bytes switched midway through reading
        // in that case, we must read again for consistency
        self.await.and_then(move |_|  {
            let start_vers = unsafe { (*self.ptr).get_write_version() };
            subroutine(Some(&self));
            if start_vers ==  unsafe { (*self.ptr).get_write_version() } {
                Ok(())
            } else {
                Err(MemError::OUT_OF_SYNC)
            }
        })
    }

    /// This function recursively calls try_visit so long as an Error is called. An Error occurs when:
    ///
    /// [A] The write version changes between the subroutine getting called and not, or;
    /// [B] ...
    #[inline]
    async fn visit_iter<Fx>(self, subroutine: Fx) -> InformationResult<'visit, ()>
        where Fx: Fn(Option<&Self>) {
        let fx_ptr = &subroutine as *const Fx;
        let self_ptr = &self as *const Self;

        while let Err(_) = unsafe  { match (&*self_ptr).try_visit(&*fx_ptr).await {
            Ok(_) => {Ok(())},
            Err(e) => {
                match e {
                    MemError::OUT_OF_SYNC => {Err(e)},
                    // Exit if there is any other type of error
                    _ => {Ok(())}
                }
            }
        } } {};

        Ok(())
    }


    /// This function will iteratively continue to seek a valid read. It ensures that, if data is changed during the subroutine's period, it will call itself again
    /// We don't do this with the writer, because the size is guaranteed to stay the same (so long as there's no illegal pointer access externally)
    pub async fn visit_until_valid_read<Fx>(self, subroutine: Fx) -> InformationResult<'visit, ()>
        where Fx: Fn(Option<&Self>) {
        self.visit_iter(subroutine).await
    }

    /// This function will read the moment its ticket becomes valid, returning reguardless if a read is valid or not.
    /// If a read was not valid, a MemoryError will return return with a reference to the corrupted bytes, just incase the user
    /// implements a design where the data needing to be read isn't dependent upon where a write simultaneously occured.
    /// In such a design, one must also account for the new length of the buffer, and as such, it is advised to not implement
    /// such a design unless the programmer knows what he/she is doing
    pub async fn visit<Fx>(self, subroutine: Fx) -> InformationResult<'visit, ()>
        where Fx: Fn(Option<&Self>) {
        self.try_visit(subroutine).await
    }


    #[inline]
    fn is_ready(&self) -> bool {
        unsafe {
            self.ticket_number == (*self.ptr).get_read_version()
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn read(&self) -> Option<&T> {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked()) }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn read_array(&self) -> Option<&[T]> where for<'a> T: Sized + 'a {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked_array()) }
        } else {
            None
        }
    }
}

impl<'visit, T: ?Sized> Future for &ReadVisitor<'visit, T> {
    type Output = InformationResult<'visit, ()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        if self.is_ready() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

/// For determining endianness of the HyperVec
#[repr(C)]
pub enum Endianness {
    /// Little Endian
    LE,
    /// Big Endian
    BE
}

impl Endianness {
    /// Determines the system endianness
    pub fn target() -> Self {
        #[cfg(target_endian = "big")]
            {
                Endianness::BE
            }
        #[cfg(not(target_endian = "big"))]
            {
                Endianness::LE
            }
    }

    /// Returns true if self is big endian
    pub fn is_be(&self) -> bool {
        match self {
            Endianness::BE => {true},
            _ => false
        }
    }

    /// Returns true if self is little endian
    #[allow(dead_code)]
    pub fn is_le(&self) -> bool {
        !self.is_be()
    }

    /// Converts a boolean value into the associated endianness
    pub fn from_bool(val: bool) -> Self {
        if val {
            Endianness::BE
        } else {
            Endianness::LE
        }
    }
}

impl Display for HyperVec {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let endianness = {
            if self.endianness.is_be(){
                "big endian (network endian) <-- most significant byte last"
            } else {
                "little endian <-- least significant byte last"
            }
        };

        write!(f, "[HyperVec] [length={}] [cursor={}] [read_version={}] [write_version={}] [Endianness={}]",
        self.len, self.cursor, self.get_read_version(), self.get_write_version(), endianness)
    }
}