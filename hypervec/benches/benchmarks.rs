#![feature(repeat_generic_slice, async_await)]
/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */

#[macro_use]
extern crate criterion;

use bytes::{Buf, BufMut, BytesMut};
use criterion::{Criterion, ParameterizedBenchmark};

use hypervec::hypervec::{HyperVec, Castable};
use futures::executor::block_on;

fn vec(len: usize, slice: &[u8]) {
    let mut mem0 = Vec::new();
    let mem = &mut mem0;

    for idx in 0..(len/slice.len()) {
        mem.put_u8(idx as u8);
    }
}

fn hyper_vec(len: usize, slice: &[u8]) {
    let mut mem0 = HyperVec::new(len).unwrap();
    let mem = &mut mem0;

    for idx in 0..(len/slice.len()) {
        mem.put_u8(idx as u8);
    }
}

fn bytes_mut(len: usize, slice: &[u8]) {
    let mut mem0 = BytesMut::with_capacity(len);
    let mem = &mut mem0;

    for idx in 0..(len/slice.len()) {
        mem.put_u8(idx as u8);
    }
}

///Main function
fn criterion_benchmark(c: &mut Criterion) {
    let slice = (0..9).collect::<Vec<u8>>();
    let slice = slice.as_ref();
    let slice = unsafe { std::mem::transmute::<&[u8], &'static [u8]>(slice) };

    /*
    c.bench(
        "Vec benches",
        ParameterizedBenchmark::new("std vec", move |b, i| b.iter(|| vec(*i as usize, slice)), vec![120])
            .with_function("HyperVec", move |b, i| b.iter(|| hyper_vec(*i as usize, slice)))
            .with_function("BytesMut", move |b, i| b.iter(|| bytes_mut(*i as usize, slice))),
    );
    */

    c.bench(
        "Lock speeds",
        ParameterizedBenchmark::new("ParkingLot::Mutex", move |b, i| b.iter(|| parkinglot_mutex()), vec![0])
            .with_function("HyperLock", move |b, i| b.iter(|| hyperlock())),
    );

}


fn hyperlock() {
    let my_x: u16 = 100;
    let mut wrapper = HyperVec::wrap(my_x).unwrap();
    let wrapper = wrapper.get_static();

    for x in 0..u16::max_value() {
        let writer = wrapper.cast_mut::<u16>().unwrap();
        let reader = wrapper.cast::<u16>().unwrap();
        block_on(writer.visit( None, |r| {
            let write = r.unwrap();
            *write.get().unwrap() = x;
            None
        })).and_then(|_| {
            reader.visit( |r| {
                let read = r.unwrap();
                let m = *read.get().unwrap();
                assert_eq!(m, x);
            });
            Ok(())
        });
    }
}

use parking_lot::RwLock;
use std::sync::Mutex;

fn parkinglot_mutex() {
    let my_x: u16 = 100;
    let mut wrapper = RwLock::new(my_x);
    for x in 0..u16::max_value() {
        block_on((async {
            *wrapper.write() = x;
        }));

        block_on((async {
            assert_eq!(*wrapper.read(), x);
        }));

    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);