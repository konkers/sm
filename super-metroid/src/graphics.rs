use failure::{format_err, Error};

use super::{Color, Palette, RoomData, RoomMdb, TileTable, TileTableEntry, Tiles, PALETTE_ENTRIES};

pub const CRE_INDEX_START: u16 = 0x280;
pub const TILE_H: usize = 8;
pub const TILE_W: usize = 8;
pub const BYTES_PER_TILE: usize = (TILE_H * TILE_W) / 2;

// SNES tiles are packed really oddly.
// From: https://mrclick.zophar.net/TilEd/download/consolegfx.txt
//
// 4BPP SNES/PC Engine
//  Colors Per Tile - 0-15
//  Space Used - 4 bits per pixel.  32 bytes for a 8x8 tile.
//
//  Note: This is a tiled, planar bitmap format.
//  Each pair represents one byte
//  Format:
//
//  [r0, bp1], [r0, bp2], [r1, bp1], [r1, bp2], [r2, bp1], [r2, bp2], [r3, bp1], [r3, bp2]
//  [r4, bp1], [r4, bp2], [r5, bp1], [r5, bp2], [r6, bp1], [r6, bp2], [r7, bp1], [r7, bp2]
//  [r0, bp3], [r0, bp4], [r1, bp3], [r1, bp4], [r2, bp3], [r2, bp4], [r3, bp3], [r3, bp4]
//  [r4, bp3], [r4, bp4], [r5, bp3], [r5, bp4], [r6, bp3], [r6, bp4], [r7, bp3], [r7, bp4]
//
//  Short Description:
//
//  Bitplanes 1 and 2 are stored first, intertwined row by row.  Then bitplanes 3 and 4
//  are stored, intertwined row by row.
fn get_pixel(data: &[u8], x: u32, y: u32) -> u8 {
    let x_shift = (7 - x) as u8;
    let mut b = 0;
    for bit in 0..4 {
        let offset = y * 2 + (bit & 0x1) + ((bit >> 1) * 16);
        if (data[offset as usize] & (1 << x_shift)) != 0 {
            b |= 1 << bit;
        }
    }
    b
}
pub fn de_planar_tiles(data: &mut [u8]) {
    const BYTES_PER_TILE: usize = (8 * 8) / 2;
    let num_tiles = data.len() / BYTES_PER_TILE;

    for tile_num in 0..num_tiles {
        let tile_data = &mut data[(tile_num as usize * BYTES_PER_TILE)..];

        let mut new_data = [0; BYTES_PER_TILE];

        for y in 0..8 {
            for x in 0..8 {
                let val = get_pixel(tile_data, x, y);
                new_data[(y * 4 + x / 2) as usize] |= if x & 0x1 == 0x1 { val << 4 } else { val }
            }
        }

        for i in 0..BYTES_PER_TILE {
            tile_data[i] = new_data[i];
        }
    }
}

pub struct TileRenderer<'a> {
    num_tiles: usize,
    cre_tiles: usize,
    sce_tiles: usize,
    graphics_sheet: Vec<u8>,
    palette: &'a Palette,
    cre_table: &'a TileTable,
    sce_table: &'a TileTable,
}

impl<'a> TileRenderer<'a> {
    pub fn new(
        cre: &Tiles,
        sce: &Tiles,
        palette: &'a Palette,
        cre_table: &'a TileTable,
        sce_table: &'a TileTable,
    ) -> Result<TileRenderer<'a>, Error> {
        let cre_tiles = cre.data.len() / BYTES_PER_TILE;
        let sce_tiles = sce.data.len() / BYTES_PER_TILE;
        let num_tiles = CRE_INDEX_START as usize + cre_tiles;

        let mut graphics_sheet = vec![0x0; num_tiles * BYTES_PER_TILE];

        for (i, b) in sce.data.iter().enumerate() {
            graphics_sheet[i] = *b;
        }

        let cre_offset = CRE_INDEX_START as usize * BYTES_PER_TILE;
        for (i, b) in cre.data.iter().enumerate() {
            graphics_sheet[cre_offset + i] = *b;
        }

        Ok(TileRenderer {
            num_tiles: num_tiles,
            cre_tiles: cre_tiles,
            sce_tiles: sce_tiles,
            graphics_sheet: graphics_sheet,
            palette: palette,
            cre_table: cre_table,
            sce_table: sce_table,
        })
    }

    pub fn get_tile(self: &Self, index: u16) -> Result<&[u8], Error> {
        if index as usize >= self.num_tiles {
            return Err(format_err!("tile {} out of range.", index));
        }

        Ok(&self.graphics_sheet[(index as usize * BYTES_PER_TILE)..])
    }

    pub fn get_pixel(data: &[u8], x: usize, y: usize) -> u8 {
        let b = data[(y * 4 + x / 2) as usize];
        if x & 0x1 == 0x1 {
            b >> 4
        } else {
            b & 0xf
        }
    }

    #[cfg(feature = "render")]
    pub fn render_tile(
        tile: &[u8],
        img: &mut image::RgbaImage,
        colors: &[Color],
        x: usize,
        y: usize,
        flip_h: bool,
        flip_v: bool,
    ) {
        assert!(colors.len() >= 16);
        for y1 in 0..TILE_H {
            for x1 in 0..TILE_W {
                let src_x = if flip_h { 7 - x1 } else { x1 };
                let src_y = if flip_v { 7 - y1 } else { y1 };
                let val = Self::get_pixel(tile, src_x, src_y);
                let color = &colors[val as usize];
                *img.get_pixel_mut((x + x1) as u32, (y + y1) as u32) =
                    image::Rgba([color.r, color.g, color.b, if val == 0 { 0x0 } else { 0xff }]);
            }
        }
    }

    #[cfg(feature = "render")]
    pub fn render_graphics_sheet(self: &Self) -> Result<image::RgbaImage, Error> {
        let tiles_w = 16;
        let tiles_h = self.num_tiles / tiles_w;
        let img_w = (tiles_w * TILE_W) as u32;
        let img_h = (tiles_h * TILE_H) as u32;

        let mut colors = Vec::new();
        for val in 0..16 {
            colors.push(Color {
                r: val << 4,
                g: val << 4,
                b: val << 4,
            });
        }

        let mut img = image::RgbaImage::new(img_w, img_h);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        for i in 0..self.sce_tiles {
            let tile = self.get_tile(i as u16)?;
            let x = i % tiles_w * 8;
            let y = i / tiles_w * 8;
            Self::render_tile(tile, &mut img, &colors, x, y, false, false);
        }

        for ci in 0..self.cre_tiles {
            let i = ci + CRE_INDEX_START as usize;
            let tile = self.get_tile(i as u16)?;
            let x = i % tiles_w * 8;
            let y = i / tiles_w * 8;
            Self::render_tile(tile, &mut img, &colors, x, y, false, false);
        }
        Ok(img)
    }

    #[cfg(feature = "render")]
    pub fn render_palette(self: &Self) -> Result<image::RgbaImage, Error> {
        let entry_w = 16;
        let entry_h = 16;
        let entries_w = 16;
        let entries_h = PALETTE_ENTRIES / entries_w;
        let img_w = entry_w * entries_w;
        let img_h = entry_h * entries_h;

        let mut img = image::RgbaImage::new(img_w as u32, img_h as u32);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        for y in 0..entries_h {
            for x in 0..entries_w {
                let entry = y * entries_w + x;
                let color = &self.palette.colors[entry];
                for y1 in 0..entry_h {
                    for x1 in 0..entry_w {
                        *img.get_pixel_mut(
                            ((x * entry_w) + x1) as u32,
                            ((y * entry_h) + y1) as u32,
                        ) = image::Rgba([color.r, color.g, color.b, 0xff]);
                    }
                }
            }
        }

        Ok(img)
    }

    #[cfg(feature = "render")]
    fn render_sub_table(
        self: &Self,
        img: &mut image::RgbaImage,
        table: &TileTable,
        offset_x: usize,
        offset_y: usize,
    ) -> Result<(), Error> {
        let tiles_w = 64;
        // a super tile is 2x2 tiles.
        let super_tiles_w = tiles_w / 2;

        for (i, entry) in table.entries.iter().enumerate() {
            let tile = self.get_tile(entry.index)?;
            let super_i = i / 4;
            let super_x = super_i % super_tiles_w * 2 * TILE_W;
            let super_y = super_i / super_tiles_w * 2 * TILE_H;
            let sub_x = i % 2;
            let sub_y = (i >> 1) & 1;
            let x = offset_x + super_x + sub_x * TILE_W;
            let y = offset_y + super_y + sub_y * TILE_H;

            Self::render_tile(
                tile,
                img,
                &self.palette.colors[(entry.palette as usize * 16)..],
                x,
                y,
                entry.flip_h,
                entry.flip_v,
            );
        }

        Ok(())
    }

    #[cfg(feature = "render")]
    pub fn render_tile_table(self: &Self) -> Result<image::RgbaImage, Error> {
        let num_entries = self.cre_table.entries.len() + self.sce_table.entries.len();
        let tiles_w = 64;
        let tiles_h = num_entries / tiles_w;
        // a super tile is 2x2 tiles.
        let img_w = (tiles_w * TILE_W) as u32;
        let img_h = (tiles_h * TILE_H) as u32;

        let mut img = image::RgbaImage::new(img_w, img_h);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        self.render_sub_table(&mut img, &self.cre_table, 0, 0)?;

        let offset_y = self.cre_table.entries.len() / tiles_w * TILE_H;
        self.render_sub_table(&mut img, &self.sce_table, 0, offset_y)?;

        Ok(img)
    }

    fn get_block_data(self: &Self, index: u16) -> Result<&[TileTableEntry], Error> {
        let tile_index = index as usize * 4;
        let cre_entries = self.cre_table.entries.len();
        let sce_entries = self.sce_table.entries.len();
        if tile_index < cre_entries {
            Ok(&self.cre_table.entries[tile_index..])
        } else if tile_index < (cre_entries + sce_entries) {
            Ok(&self.sce_table.entries[(tile_index - cre_entries)..])
        } else {
            Err(format_err!("tile index 0x{:x} out of range", index))
        }
    }

    #[cfg(feature = "render")]
    pub fn render_block(
        self: &Self,
        img: &mut image::RgbaImage,
        index: u16,
        x: usize,
        y: usize,
        flip_h: bool,
        flip_v: bool,
    ) -> Result<(), Error> {
        let block_data = self.get_block_data(index)?;

        // there are the destination offsets for the x and y cooredinates of
        // each sub tile.
        let x_offsets = if flip_h { [8, 0, 8, 0] } else { [0, 8, 0, 8] };
        let y_offsets = if flip_v { [8, 8, 0, 0] } else { [0, 0, 8, 8] };

        for sub_tile in 0..4 {
            let entry = &block_data[sub_tile]; //x_offsets[sub_tile].0 + y_offsets[sub_tile].0];
            let tile = self.get_tile(entry.index)?;
            Self::render_tile(
                tile,
                img,
                &self.palette.colors[(entry.palette as usize * 16)..],
                x + x_offsets[sub_tile],
                y + y_offsets[sub_tile],
                entry.flip_h ^ flip_h,
                entry.flip_v ^ flip_v,
            );
        }

        Ok(())
    }

    #[cfg(feature = "render")]
    pub fn render_room(
        self: &Self,
        state: usize,
        mdb: &RoomMdb,
        data: &RoomData,
    ) -> Result<image::RgbaImage, Error> {
        let block_h = 16;
        let block_w = 16;
        let super_block_h = 16;
        let super_block_w = 16;
        let img_w = (mdb.width as usize * (super_block_w * block_w)) as u32;
        let img_h = (mdb.height as usize * (super_block_h * block_h)) as u32;
        let room_blocks_w = mdb.width as usize * super_block_w;
        let room_blocks_h = mdb.height as usize * super_block_h;

        let mut img = image::RgbaImage::new(img_w, img_h);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        for (i, block) in data.layer_1.iter().enumerate() {
            let x = i % room_blocks_w;
            let y = i / room_blocks_w;
            if y >= room_blocks_h as usize {
                // Bowling Alley and Double Chamber have too much data.  It is
                // unclear why.
                break;
            }
            self.render_block(
                &mut img,
                block.tile_index,
                x * block_w,
                y * block_h,
                block.x_flip,
                block.y_flip,
            )?;
        }
        Ok(img)
    }
}
