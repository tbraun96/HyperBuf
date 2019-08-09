/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution. 
 */

use std::alloc::{Alloc, Layout};
use std::ops::{Index, IndexMut, Range};
use std::ptr::NonNull;

use bytes::BufMut;

use crate::results::{InformationResult, MemError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::marker::PhantomData;

/// This is a type which can be re-interpreted to any type, regardless of alignment
#[fundamental]
#[repr(C)]
pub struct HyperVec {
    ptr: *mut u8,
    len: usize,
    cursor: isize,
    /// The read and write versions are only for editing data through visitors
    read_version: AtomicUsize,
    write_version: AtomicUsize,
    /// See [WriteVisitor] for the definition of "corrupt", as we
    corrupt: bool,
    /// We place the layout at the end of the struct to ensure that, in the event of corruption, the bytes do not interfere with this struct.
    layout: Layout
}

/// The primary HyperVec is allowed to ship around between threads
unsafe impl Send for HyperVec {}
/// Data races can be fully prevented by using WriteVisitors; However, these devices cannot be shipped between threads, and should only be used for future writes
unsafe impl Sync for HyperVec {}

impl Drop for HyperVec {
    fn drop(&mut self) {
        unsafe { std::alloc::Global.dealloc(NonNull::new(self.ptr).unwrap(), self.layout) }
    }
}

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

impl IntoIterator for HyperVec {
    type Item = u8;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let len = self.len as isize;
        self[0..len].to_vec().into_iter()
    }
}

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
    #[expand(depth=5,expression="*self.ptr.offset(p0 + {}) = slice[{}]")]
    fn put_slice(&mut self, slice: &[u8]) {
        unsafe {
            debug_assert!(self.remaining_mut() >= slice.len());
            let p0 = self.cursor;
            let len = slice.len() as isize;
            match len {

            }
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
    fn cast<Type>(&self)  -> InformationResult<ReadVisitor<Type>>;
    /// Casts the underlying bytes to an immutable version of the the supplied type without checking alignment
    unsafe fn cast_unchecked<Type>(&self)  -> &Type;
    /// Casts the underlying bytes to a mutable version of the supplied type with checking alignment accompanied by a WriteVisitor
    fn cast_mut<Type>(&mut self)  -> InformationResult<WriteVisitor<Type>>;
    /// Casts the underlying bytes to a mutable version of the the supplied type without checking alignment
    unsafe fn cast_unchecked_mut<Type>(&mut self) -> &mut Type;
}

impl Castable for HyperVec {
    fn cast<Type>(&self) -> InformationResult<ReadVisitor<Type>> {
        if std::mem::align_of::<Type>() == self.layout.size() {
            Ok(ReadVisitor::new((&*self as *const Self) as *mut Self, self.get_read_version()))
        } else {
            MemError::throw_corrupt(&"Invalid alignment")
        }
    }


    unsafe fn cast_unchecked<Type>(&self) -> &Type {
        &*(self.ptr as *mut Type)
    }

    fn cast_mut<Type>(&mut self) -> InformationResult<WriteVisitor<Type>> {
        if std::mem::align_of::<Type>() == self.layout.size() {
            Ok(WriteVisitor::new(&mut *self as *mut Self, self.get_write_version()))
        } else {
            MemError::throw_corrupt(&"Invalid alignment")
        }
    }

    unsafe fn cast_unchecked_mut<Type>(&mut self) -> &mut Type {
        &mut *(self.ptr as *mut Type)
    }

}


impl HyperVec {
    #[inline]
    /// Returns a HyperVec module that is blocked
    pub fn new(len: usize) -> Self {
        let layout = unsafe { Layout::from_size_align_unchecked(len, 1) };
        let ptr = unsafe { std::alloc::alloc(layout) };
        Self { ptr, len, layout, cursor: 0, read_version: AtomicUsize::new(0), write_version: AtomicUsize::new(0), corrupt: false }
    }

    #[inline]
    /// Returns a HyperVec module that is blocked
    pub fn new_zeroed(len: usize) -> Self {
        let layout = Layout::array::<u8>(len).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        Self { ptr, len, layout, cursor: 0, read_version: AtomicUsize::new(0), write_version: AtomicUsize::new(0), corrupt: false }
    }

    #[inline]
    /// Wraps around a pre-existing value, translating it into its bytes
    pub fn wrap<T: Sized>(t: T) -> Self {
        let ptr0 = (&t as *const T) as *const u8;
        let layout = Layout::for_value(&t);
        let ptr = unsafe{ std::alloc::alloc(layout) };

        unsafe { std::ptr::copy_nonoverlapping(ptr0, ptr, layout.size()) };

        Self {
            ptr,
            len: layout.size(),
            cursor: 0,
            read_version: AtomicUsize::new(0),
            write_version: AtomicUsize::new(0),
            corrupt: false,
            layout
        }
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

    /// Pretty damn unsafe
    #[inline]
    pub unsafe fn get_and_increment_read_version(&self) -> usize {
        (*self).read_version.fetch_add(1, Ordering::SeqCst)
    }

    /// Pretty damn unsafe
    #[inline]
    pub unsafe fn get_and_increment_write_version(&self) -> usize {
        (*self).write_version.fetch_add(1, Ordering::SeqCst)
    }

    /*
    /// converts immutable self to mutable self
    unsafe fn rip_mut(&self) -> *mut Self {
        (&*self as *const Self) as *mut Self
    }*/

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
            assert_eq!(layout.size(), additional_bytes);
            self.len = self.len + layout.size();
            self.ptr = unsafe { std::alloc::realloc(self.ptr, layout, self.len) };
        }
    }
}

/// Allows asynchronous data execution once it's spot in line reaches the 'front'.
pub struct WriteVisitor<'visit, T> {
    ptr: *mut HyperVec,
    ticket_number: usize,
    bytes_written: usize,
    _phantom: PhantomData<&'visit T>
}

unsafe impl<'visit, T> Send for WriteVisitor<'visit, T> {}
unsafe impl<'visit, T > Sync for WriteVisitor<'visit, T> {}

#[allow(unused_results)]
impl<'visit, T> Drop for WriteVisitor<'visit, T> {
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

impl<'visit, T> WriteVisitor<'visit, T> {
    /// Creates a new WriteVisitor
    pub fn new(hvec_ptr: *mut HyperVec, ticket_number: usize) -> Self {
        Self { ptr: hvec_ptr, ticket_number, _phantom: PhantomData, bytes_written: 0}
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
    /// [2]
    #[inline]
    pub async fn visit<Fx>(self, pre_alloc: Option<usize>, subroutine: Fx) -> InformationResult<'visit, ()> where Fx: Fn(Option<&Self>) -> Option<usize>{
        if let Some(alloc) = pre_alloc {
            unsafe { (*(self).ptr).extend(alloc) };
        }

        (&self).await.and_then(move |_| {
            unsafe {
                //println!("Will exec subroutine {}", self.ticket_number);
                let initial_size = (*(self).ptr).len;
                let pre_alloc_amt = pre_alloc.unwrap_or(0);

                match subroutine(Some(&self)) {
                    Some(bytes_added) => {
                        if bytes_added > initial_size + pre_alloc_amt {
                            (*(self).ptr).corrupt = true;
                            let bytes = (*self.ptr).get_full_bytes_mut();
                            MemError::throw_corrupt(bytes)
                        } else {
                            //println!("Valid write! {} bytes added", bytes_added);
                            Ok(())
                        }
                    },

                    None => {
                        //println!("Valid write!");
                        Ok(())
                    }
                }
            }
        })
    }

    #[inline]
    fn is_ready(&self) -> bool {
        unsafe {
            self.ticket_number == (*self.ptr).get_write_version()
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn get(&self) -> Option<&mut T> {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked_mut()) }
        } else {
            None
        }
    }

}

use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;

impl<'visit, T> Future for &WriteVisitor<'visit, T> {
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
pub struct ReadVisitor<'visit, T> {
    ptr: *mut HyperVec,
    ticket_number: usize,
    bytes_written: usize,
    _phantom: PhantomData<&'visit T>
}

unsafe impl<'visit, T> Send for ReadVisitor<'visit, T> {}
unsafe impl<'visit, T> Sync for ReadVisitor<'visit, T> {}

#[allow(unused_results)]
impl<'visit, T> Drop for ReadVisitor<'visit, T> {
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

impl<'visit, T> ReadVisitor<'visit, T> {
    /// Creates a new WriteVisitor
    pub fn new(hvec_ptr: *mut HyperVec, ticket_number: usize) -> Self {
        Self { ptr: hvec_ptr, ticket_number, _phantom: PhantomData, bytes_written: 0}
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
    /// [2]
    #[inline]
    pub async fn visit<Fx>(self, subroutine: Fx) -> InformationResult<'visit, ()> where Fx: Fn(Option<&Self>) {
        (&self).await.and_then(move |_| {
                //let no = (self).ticket_number;
                //println!("Will exec subroutine {}", no);
                let _ = subroutine(Some(&self));
                Ok(())
        })
    }

    #[inline]
    fn is_ready(&self) -> bool {
        unsafe {
            self.ticket_number == (*self.ptr).get_read_version()
        }
    }

    /// Returns a mutable reference to the underlying object if available
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_ready() {
            unsafe { Some((*self.ptr).cast_unchecked()) }
        } else {
            None
        }
    }

}

impl<'visit, T> Future for &ReadVisitor<'visit, T> {
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