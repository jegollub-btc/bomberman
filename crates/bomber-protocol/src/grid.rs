//! Packing the board into bytes.
//!
//! Two bits per cell, row-major, low bits first. A 15x13 board is 49 bytes and
//! a 31x31 board is 241, which is what lets a keyframe carry the *whole* grid
//! instead of a patch list -- simpler to apply and bounded in size, where a
//! patch list grows all match long.

use bomber_domain::board::{Tile, TileGrid};

use crate::codec::{ProtoError, Reader, Result, Writer};

pub fn packed_len(width: u8, height: u8) -> usize {
    (width as usize * height as usize).div_ceil(4)
}

pub fn encode(grid: &TileGrid, w: &mut Writer) {
    let mut packed = vec![0u8; packed_len(grid.width(), grid.height())];
    for (i, tile) in grid.cells().iter().enumerate() {
        packed[i / 4] |= tile.code() << ((i % 4) * 2);
    }
    w.bytes(&packed);
}

pub fn decode(r: &mut Reader, width: u8, height: u8) -> Result<TileGrid> {
    let count = width as usize * height as usize;
    let packed = r.bytes(packed_len(width, height))?;
    let mut cells = Vec::with_capacity(count);
    for i in 0..count {
        let code = (packed[i / 4] >> ((i % 4) * 2)) & 0b11;
        cells.push(Tile::from_code(code).ok_or_else(|| ProtoError::invalid("tile", code))?);
    }
    TileGrid::from_cells(width, height, cells).ok_or_else(|| ProtoError::invalid("grid", 0u8))
}
