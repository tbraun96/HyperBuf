/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */

use std::fmt::{Debug, Display, Formatter};
use std::error::Error;


/// #
#[allow(non_camel_case_types)]
pub enum MemError<'a, T: AsRef<[u8]>> {
    CORRUPT(Vec<u8>),
    _phantom(&'a T)
}

impl<'a, T: AsRef<[u8]> + 'a> MemError<'a, T> {
    pub fn throw_corrupt<U>(symbol: T) -> InformationResult<'a, U> {
        Err(MemError::CORRUPT(symbol.as_ref().to_vec()))
    }

    fn printf(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            MemError::CORRUPT(t) => {
                write!(f, "[MemoryError] {}", String::from_utf8_lossy(t.as_slice()))
            }
            _ => {write!(f, "[MemoryError] Undefin")}
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

/// #
pub type InformationResult<'a,T> = Result<T, MemError<'a, &'a [u8]>>;
