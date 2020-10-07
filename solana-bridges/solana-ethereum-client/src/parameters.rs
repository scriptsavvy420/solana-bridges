use std::mem;
use std::num::Wrapping;

use solana_sdk::program_pack::{Pack};

use crate::eth::BlockHeader;

pub const HEADER_HISTORY_SIZE: usize = 100;

pub const BLOCKS_OFFSET: usize = mem::size_of::<usize>() + mem::size_of::<u64>() + 1; // TODO better
pub const MIN_BUF_SIZE: usize = BLOCKS_OFFSET + BlockHeader::LEN;

pub const STORAGE_ALIGN: usize = std::mem::align_of::<StorageScrach>();

#[repr(C)]
pub struct StorageT<X: ?Sized> {
    pub height: u64,
    pub offset: Wrapping<usize>,
    pub full: bool,
    pub headers: X,
}

pub type Storage = StorageT<[[u8; BlockHeader::LEN]]>;

// Something sized that can be unsized, useful for some compile time math
pub type StorageScrach = StorageT<[[u8; BlockHeader::LEN]; 5]>;
