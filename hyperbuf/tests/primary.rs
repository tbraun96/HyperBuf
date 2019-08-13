#![feature(async_await)]
/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution. 
 */

#[macro_use]
extern crate hyxe_derive;

use std::mem::size_of_val;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bytes::{BigEndian, ByteOrder, LittleEndian};
use bytes::BufMut;
use futures::{Poll, TryFutureExt};
use futures::executor::block_on;
use parking_lot::Mutex;

use hypervec::hypervec::{HyperVec, WriteVisitor};

use hypervec::impls::Castable;
use hypervec::prelude::ByteWrapper;

#[test]
fn vectors(){
    let file_pos = "C:\\Users\\tbrau\\test.h";
    let items = &[10 as u8, 3, 99, 255, 251, 254];
    let mut wrapper = HyperVec::wrap_bytes(items);
    println!("len: {}", wrapper.length());
    let wrapper_ref = &mut wrapper;
    for byte in wrapper_ref {
        println!("{}", unsafe {*byte});
    }

    let _ = wrapper.serialize_to_disk(file_pos);

    let wrapper2 = block_on(HyperVec::deserialize_from_disk(file_pos)).unwrap();
    println!("len: {}", wrapper2.length());
    for byte in wrapper2 {
        println!("{}", unsafe {*byte});
    }
    //wrapper.serialize_to_disk().unwrap();
}

#[test]
fn test_dynamic_memory2() {
    let init = Instant::now();
    let my_x: u16 = 100;
    let mut wrapper = HyperVec::wrap(my_x);

    for x in 0..u16::max_value() {
        let writer = wrapper.cast_mut::<u16>().unwrap();

        let subroutine = |r: Option<&WriteVisitor<u16>>| {
            let write = r.unwrap();
            *write.get().unwrap() = x;
            None
        };

        let _ = block_on(writer.visit(None, subroutine));

        let reader = wrapper.cast::<u16>().unwrap();
        let _ = reader.try_visit(|r| {
            let read = r.unwrap();
            let m = *read.get().unwrap();
            assert_eq!(m, x);
        });
    }

    let end = Instant::now();
    let diff = end - init;
    println!("{}s {}ns", diff.as_secs(), diff.subsec_nanos());

    for byte in wrapper.into_iter() {
        println!("{}", unsafe {*byte});
    }
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