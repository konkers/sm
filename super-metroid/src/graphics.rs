use failure::{format_err, Error};

use super::Tiles;

pub const CRE_INDEX_START: u16 = 0x280;
pub const TILE_H: usize = 8;
pub const TILE_W: usize = 8;
pub const BYTES_PER_TILE: usize = (TILE_H * TILE_W) / 2;

pub struct TileRenderer {
    num_tiles: usize,
    cre_tiles: usize,
    sce_tiles: usize,
    graphics_sheet: Vec<u8>,
}

impl TileRenderer {
    pub fn new(cre: &Tiles, sce: &Tiles) -> Result<TileRenderer, Error> {
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
    pub fn render_tile(tile: &[u8], img: &mut image::RgbaImage, x: usize, y: usize) {
        for y1 in 0..TILE_H {
            for x1 in 0..TILE_W {
                let val = Self::get_pixel(tile, x1, y1) << 4;
                *img.get_pixel_mut((x + x1) as u32, (y + y1) as u32) =
                    image::Rgba([val, val, val, 0xff]);
            }
        }
    }

    #[cfg(feature = "render")]
    pub fn render_graphics_sheet(self: &Self) -> Result<image::RgbaImage, Error> {
        let tiles_w = 16;
        let tiles_h = self.num_tiles / tiles_w;
        let img_w = (tiles_w * TILE_W) as u32;
        let img_h = (tiles_h * TILE_H) as u32;

        let mut img = image::RgbaImage::new(img_w, img_h);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        for i in 0..self.sce_tiles {
            let tile = self.get_tile(i as u16)?;
            let x = i % tiles_w * 8;
            let y = i / tiles_w * 8;
            Self::render_tile(tile, &mut img, x, y);
        }

        for ci in 0..self.cre_tiles {
            let i = ci + CRE_INDEX_START as usize;
            let tile = self.get_tile(i as u16)?;
            let x = i % tiles_w * 8;
            let y = i / tiles_w * 8;
            Self::render_tile(tile, &mut img, x, y);
        }
        Ok(img)
    }
}
