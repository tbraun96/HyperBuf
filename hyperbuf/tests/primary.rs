#![feature(async_await, slice_from_raw_parts)]
/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution. 
 */

#[macro_use]
extern crate hyperbuf_derive;

use std::mem::size_of_val;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bytes::{BigEndian, ByteOrder, LittleEndian};
use bytes::BufMut;
use futures::{Poll, TryFutureExt};
use futures::executor::block_on;
use parking_lot::Mutex;

use hyperbuf::hypervec::{HyperVec, WriteVisitor};

use hyperbuf::impls::Castable;
use hyperbuf::prelude::{ByteWrapper, BytePusher};
use std::fmt::{Display, Formatter, Error};
use std::marker::PhantomData;

pub struct Txx {
    field: u8,
    field2: u16,
    field3: u32,
    field4: u16
}

fn zero_alloc(txx: &Txx) -> (*mut u8, usize) {
    unsafe {
        let layout = std::alloc::Layout::from_size_align_unchecked(9, 0);
        (std::alloc::alloc(layout), 9)
    }
}


impl Txx {
    pub fn new(seed: usize) -> Self {
        Self {field: (seed + 10) as u8, field2: (seed + 111) as u16, field3: (seed + 222) as u32, field4: seed as u16}
    }
}

impl Display for Txx {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{} {} {} -> SEED {}", self.field, self.field2, self.field3, self.field4)
    }
}

fn print_all(txx: &Txx) {
    let ptr = txx as *const Txx as *const u8;
    let size = std::mem::size_of::<Txx>();

    for x in 0..(size as isize) {
        println!("TXX_VAL {}", unsafe { *ptr.offset(x) });
    }
}



#[test]
fn vectors(){
    let file_pos = "C:\\Users\\tbrau\\test.h";
    let i0 = Txx::new(0);
    let i1 = Txx::new(1);
    let i2 = Txx::new(10);

    let items = &[i0, i1, i2];
    //println!("SizeOF Tx: {}, AlignOf {}", std::mem::size_of::<Txx>(), std::mem::align_of::<Txx>());

    let mut wrapper = HyperVec::wrap(items);
    println!("pushing");
    //wrapper.set_cursor_pos(3);
    //wrapper.push_u16s(items);
    println!("{}", wrapper);
    let wrapper_ref = wrapper.as_static();


    /*for byte in wrapper_ref {
        println!("{}", unsafe {*byte});
    }*/
    let write = wrapper_ref.cast_mut::<Txx>().unwrap();
    block_on(write.visit(None, |write| unsafe {
        let m = write.get_array()?;
        let mut i = 0;

        let v1 = &m[0];
        let v2 = &m[1];
        let v3 = &m[2];
        println!("RECOMBINE {} {} {}", v1, v2, v3);
        for x in m {
            println!("[{}] ptr: {}", i, x);
            i+=1;
        }
        None
    })).unwrap();

    println!("{}", &wrapper[0]);

    let _ = wrapper.serialize_to_disk(file_pos);

    let mut wrapper2 = block_on(HyperVec::deserialize_from_disk(file_pos)).unwrap();

    wrapper2.reset_cursor();
    println!("W2 {}", wrapper2);
    println!("len: {}", wrapper2.length());
    for byte in wrapper2 {
        println!("=> {}", byte);
    }
    //wrapper.serialize_to_disk().unwrap();
}

#[test]
fn test_dynamic_memory() {
    let my_x: u16 = 100;
    let mut wrapper = Mutex::new(my_x);
    for x in 0..u16::max_value() {
        let _ = block_on(async {
            *wrapper.lock() = x;
        });

        let _ = block_on(async {
            assert_eq!(*wrapper.lock(), x);
        });
    }
}