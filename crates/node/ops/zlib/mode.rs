// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use libz_sys as sys;

#[derive(Debug)]
pub enum Error {
    BadArgument,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadArgument => write!(f, "bad argument"),
        }
    }
}

impl std::error::Error for Error {}

macro_rules! repr_i32 {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
      $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
      $(#[$meta])*
      $vis enum $name {
        $($(#[$vmeta])* $vname $(= $val)?,)*
      }

      impl core::convert::TryFrom<i32> for $name {
        type Error = Error;

        fn try_from(v: i32) -> Result<Self, Self::Error> {
          match v {
            $(x if x == $name::$vname as i32 => Ok($name::$vname),)*
            _ => Err(Error::BadArgument),
          }
        }
      }
    }
  }

repr_i32! {
  #[repr(i32)]
  #[derive(Clone, Copy, Debug, PartialEq, Default)]
  pub enum Mode {
    #[default]
    None,
    Deflate,
    Inflate,
    Gzip,
    Gunzip,
    DeflateRaw,
    InflateRaw,
    Unzip,
  }
}

repr_i32! {
  #[repr(i32)]
  #[derive(Clone, Copy, Debug, PartialEq, Default)]
  pub enum Flush {
    #[default]
    None = sys::Z_NO_FLUSH,
    Partial = sys::Z_PARTIAL_FLUSH,
    Sync = sys::Z_SYNC_FLUSH,
    Full = sys::Z_FULL_FLUSH,
    Finish = sys::Z_FINISH,
    Block = sys::Z_BLOCK,
    Trees = sys::Z_TREES,
  }
}
