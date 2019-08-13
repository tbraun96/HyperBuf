/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */

use std::fmt::{Debug, Display, Formatter};
use std::error::Error;


/// #
#[allow(non_camel_case_types)]
pub enum MemError<'a, T: AsRef<[u8]>> {
    /// Contains the corrupted bytes
    CORRUPT(Vec<u8>),
    /// Out of sync
    OUT_OF_SYNC,
    /// Not ready (for polling)
    NOT_READY,
    /// A generic error message
    GENERIC(T),
    /// phantom
    _phantom(&'a T)
}

impl<'a, T: AsRef<[u8]> + 'a> MemError<'a, T> {

    /// #
    pub fn throw_corrupt<U>(symbol: T) -> InformationResult<'a, U> {
        Err(MemError::CORRUPT(symbol.as_ref().to_vec()))
    }

    /// #
    pub fn throw<U>(data: &'a T) -> InformationResult<'a, U> {
        Err(MemError::GENERIC(data.as_ref()))
    }

    fn printf(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            MemError::CORRUPT(t) => {
                write!(f, "[MemoryError] {}", String::from_utf8_lossy(t.as_slice()))
            },

            MemError::OUT_OF_SYNC => {
                write!(f, "[MemoryError] Out of Sync")
            },

            MemError::NOT_READY => {
                write!(f, "[MemoryError] Not ready")
            },

            MemError::GENERIC(msg) => {
                write!(f, "[MemoryError] {}", String::from_utf8_lossy((*msg.as_ref()).as_ref()))
            }
            _ => {write!(f, "[MemoryError] Undefined")}
        }
    }

    fn value(&self) -> i32 {
        match self {
            MemError::CORRUPT(_) => {
                0
            },

            MemError::OUT_OF_SYNC => {
                1
            },

            MemError::NOT_READY => {
                2
            },

            MemError::GENERIC(_) => {
                3
            }
            _ => {4}
        }
    }
}

impl<'a, T: AsRef<[u8]>> Display for MemError<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        self.printf(f)
    }
}

impl<'a, T: AsRef<[u8]>> Debug for MemError<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        self.printf(f)
    }
}

impl<'a, T: AsRef<[u8]>> Error for MemError<'a, T> {}

impl<'a, T: AsRef<[u8]> + Send + Sync> Into<std::io::Error> for MemError<'a, T> {
    fn into(self) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, std::io::Error::from_raw_os_error(self.value()))
    }
}

/// #
pub type InformationResult<'a,T> = Result<T, MemError<'a, &'a [u8]>>;
