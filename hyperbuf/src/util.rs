/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */

/// For efficient packing of data
#[allow(unused)]
pub mod bit_handler {
    /// for shifting
    const EMPTY5: u8 = 0b1111_0000;
    /// for shifting
    const EMPTY6: u8 = 0b0000_1111;

    #[inline]
    /// Packs two values (n,k) such that 0 <= (n,k) <= 2^4 into a single 8-bit byte. There are NO CHECKS IF `first` or `second` are above this for performance reasons! Use wisely!
    pub fn pack4_4(first: u8, second: u8) -> u8 {
        (first << 4) | second
    }

    #[inline]
    /// The inverse of pack4_4. Returns the values in the original order they were packed
    pub fn unpack4_4(byte: u8) -> [u8; 2] { [(byte & EMPTY5) >> 4, byte & EMPTY6] }

    #[repr(align(4))]
    /// Used for storing powers of two
    #[allow(missing_docs)]
    pub enum U4 {
        ONE = 0b0001,
        TWO = 0b0010,
        THREE = 0b0011,
        FOUR = 0b0100,
        FIVE = 0b0101,
        SIX = 0b0110,
        SEVEN = 0b0111,
    }
}

pub(super) mod ser {
    use std::fs::File;
    //use tokio::fs::File;
    //use futures::TryFutureExt;
    //use tokio::io::AsyncWriteExt;
    use std::io::{BufReader, Write};
    use crate::results::MemError;
    use crate::impls::HyperVecSerde;
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    /// Serializes an entity to the disk
    pub(crate) fn serialize_hypervec_to_disk<T: Serialize>(full_path: &str, entity: &T) -> Result<usize, std::io::Error> {
        //bincode::serialize(entity).unwrap().as_slice()
        File::create(full_path)
            .and_then(|mut file| file.write(bincode::serialize(entity).unwrap().as_slice()))
            .map_err(|err| err)
    }

    /// Deserializes an entity to the disk
    /// Objects to consider:
    ///             bytes,
    ///             cursor (isize: 8 bytes),
    ///             read_version (usize: 8 bytes),
    ///             write_version (usize: 8 bytes),
    ///             is_be (bool: 1 byte)
    /// Tactic: start from the end, assume the bytes are properly placed in order
    pub(crate) fn deserialize_hypervec_from_disk<T: DeserializeOwned>(full_path: &str) -> Result<T, std::io::Error> {
        //bincode::config().deserialize_from(rx).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, MemError::<String>::GENERIC(err.to_string()))
        File::open(full_path).and_then(|res| {
            let rx = BufReader::new(res);
            bincode::config().deserialize_from(rx).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, MemError::<String>::GENERIC(err.to_string())))
        })
    }

    static HYPERVEC_MIN_SIZE: usize = 25;

    #[allow(unused)]
    pub fn ptr_deserialize_hypervecserde(bytes: &[u8]) -> Result<HyperVecSerde, std::io::Error> {
        let len = bytes.len();
        if len < HYPERVEC_MIN_SIZE {
            Err(std::io::Error::new(std::io::ErrorKind::Other, MemError::<String>::GENERIC("Invalid size!".to_string())))
        } else {
            let is_be = bytes[len - 1] == 1;
            let write_version = usize::from_le_bytes([bytes[(len - 9)], bytes[(len - 8)], bytes[(len - 7)], bytes[(len - 6)], bytes[(len - 5)], bytes[(len - 4)], bytes[(len - 3)], bytes[(len - 2)]]);
            let read_version = usize::from_le_bytes([bytes[(len - 17)], bytes[(len - 16)], bytes[(len - 15)], bytes[(len - 14)], bytes[(len - 13)], bytes[(len - 12)], bytes[(len - 11)], bytes[(len - 10)]]);
            let cursor = isize::from_le_bytes([bytes[(len - 25)], bytes[(len - 24)], bytes[(len - 23)], bytes[(len - 22)], bytes[(len - 21)], bytes[(len - 20)], bytes[(len - 19)], bytes[(len - 18)]]);
            let bytes = &bytes[0..(len - 26)];
            Ok(HyperVecSerde(bytes.to_vec(), cursor, read_version, write_version, is_be))
        }
    }

    #[allow(unused)]
    pub fn ptr_serialize<T: Sized>(t: &T) -> Box<[u8]> {
        let size = std::mem::size_of_val(&t);
        println!("Will serialize {} bytes", size);
        let mut bytes = Vec::<u8>::with_capacity(size);
        let ptr = t as *const T;
        let ptr = ptr as *const u8;

        for idx in 0..(size as isize) {
            unsafe { bytes.push(*ptr.offset(idx)) };
        }

        bytes.into_boxed_slice()
    }

}