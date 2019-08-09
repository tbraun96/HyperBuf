#![feature(async_await)]
/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution. 
 */

use bytes::BufMut;
use hypervec::hypervec::{HyperVec, Castable, WriteVisitor};
use std::time::{Instant, Duration};
use std::thread;
use std::thread::JoinHandle;
use futures::executor::block_on;
use std::mem::size_of_val;
use bytes::{BigEndian, ByteOrder};
use parking_lot::Mutex;
use futures::{TryFutureExt, Poll};

#[macro_use]
extern crate hyxe_derive;

#[test]
fn test_dynamic_memory2() {
    let my_x: u16 = 100;
    let mut wrapper = HyperVec::wrap(my_x);

    for x in 0..u16::max_value() {
        let writer = wrapper.cast_mut::<u16>().unwrap();

            let _ = block_on(writer.visit( None, |r| {
                let write = r.unwrap();
                *write.get().unwrap() = x;
                None
            })).and_then(|_| {
                /*let reader = wrapper.cast::<u16>().unwrap();
                let _ = reader.visit( |r| {
                    let read = r.unwrap();
                    let m = *read.get().unwrap();
                    assert_eq!(m, x);
                });*/
                Ok(())
            });
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