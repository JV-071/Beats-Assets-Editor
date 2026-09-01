use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const SPRITE_WIDTH: u32 = 32;
pub const SPRITE_HEIGHT: u32 = 32;
pub const SPRITE_PIXELS: usize = (SPRITE_WIDTH * SPRITE_HEIGHT) as usize; // 1024
pub const SPRITE_RGBA_BYTES: usize = SPRITE_PIXELS * 4; // 4096

/// Represents a single decoded 32x32 RGBA sprite
#[derive(Debug, Clone)]
pub struct DecodedSprite {
    pub id: u32,
    pub rgba: Vec<u8>,
}

/// Parsed legacy SPR file structure with offsets — file data is read on-demand from disk.
/// Only stores the path + offset table (~4.5MB for 1.12M sprites) instead of the entire
/// file (~500MB-1GB), preventing OOM kills on low-RAM machines.
pub struct LegacySprReader {
    pub signature: u32,
    pub sprite_count: u32,
    pub is_extended: bool,
    pub is_transparent: bool,
    pub offsets: Vec<u32>,
    file_path: PathBuf,
    file_len: u64,
}

impl LegacySprReader {
    /// Opens and parses header + offset table of a legacy .spr file.
    /// Does NOT load the file content into RAM — sprites are read on-demand.
    pub fn open<P: AsRef<Path>>(path: P, is_extended: bool, is_transparent: bool) -> Result<Self> {
        let path = path.as_ref();
        let mut file = BufReader::new(
            File::open(path).context(format!("Failed to open SPR file: {:?}", path))?
        );
        let file_len = file.get_ref().metadata()?.len();

        if file_len < 6 {
            return Err(anyhow!("SPR file too small (less than 6 bytes)"));
        }

        let signature = file.read_u32::<LittleEndian>().context("Failed to read SPR signature")?;

        let (sprite_count, header_size) = if is_extended {
            if file_len < 8 {
                return Err(anyhow!("Extended SPR file too small (less than 8 bytes)"));
            }
            let count = file.read_u32::<LittleEndian>().context("Failed to read u32 sprite count")?;
            (count, 8u64)
        } else {
            let count = file.read_u16::<LittleEndian>().context("Failed to read u16 sprite count")? as u32;
            (count, 6u64)
        };

        let required_bytes = header_size + (sprite_count as u64 * 4);
        if file_len < required_bytes {
            return Err(anyhow!(
                "SPR file truncated: expected at least {} bytes for {} sprite offsets, but file has {} bytes",
                required_bytes,
                sprite_count,
                file_len
            ));
        }

        let mut offsets = Vec::with_capacity(sprite_count as usize);
        for _ in 0..sprite_count {
            offsets.push(file.read_u32::<LittleEndian>()?);
        }

        Ok(Self {
            signature,
            sprite_count,
            is_extended,
            is_transparent,
            offsets,
            file_path: path.to_path_buf(),
            file_len,
        })
    }

    /// Decodes a single sprite by ID (1-based index)
    pub fn decode_sprite(&self, id: u32) -> Result<DecodedSprite> {
        if id == 0 || id > self.sprite_count {
            return Err(anyhow!("Sprite ID {} out of range (1..{})", id, self.sprite_count));
        }

        let offset = self.offsets[(id - 1) as usize];
        let mut file = BufReader::new(
            File::open(&self.file_path).context("Failed to reopen SPR file for sprite decode")?
        );
        let rgba = decode_sprite_rle_from_reader(&mut file, offset, self.file_len, self.is_transparent)?;

        Ok(DecodedSprite { id, rgba })
    }

    /// Decodes a batch of sprites by range (1-based indices) reading from disk on-demand.
    /// Sprites that fail to decode are replaced with transparent (empty) sprites.
    pub fn decode_batch(&self, start_id: u32, count: u32) -> Result<Vec<DecodedSprite>> {
        let end_id = (start_id.saturating_add(count).saturating_sub(1)).min(self.sprite_count);
        if start_id == 0 || start_id > end_id {
            return Ok(Vec::new());
        }

        let mut file = BufReader::new(
            File::open(&self.file_path).context("Failed to reopen SPR file for batch decode")?
        );

        let mut sprites = Vec::with_capacity((end_id - start_id + 1) as usize);
        for id in start_id..=end_id {
            let offset = self.offsets[(id - 1) as usize];
            match decode_sprite_rle_from_reader(&mut file, offset, self.file_len, self.is_transparent) {
                Ok(rgba) => sprites.push(DecodedSprite { id, rgba }),
                Err(_e) => {
                    sprites.push(DecodedSprite { id, rgba: vec![0u8; SPRITE_RGBA_BYTES] });
                }
            }
        }

        Ok(sprites)
    }

    /// Decodes all sprites sequentially (1-based indices from 1 to sprite_count)
    pub fn decode_all_sprites(&self) -> Result<Vec<DecodedSprite>> {
        self.decode_batch(1, self.sprite_count)
    }
}

/// Decodes RLE-compressed 32x32 sprite from a file reader at a specific offset
fn decode_sprite_rle_from_reader<R: Read + Seek>(
    reader: &mut R,
    offset: u32,
    file_len: u64,
    is_transparent: bool,
) -> Result<Vec<u8>> {
    let mut output = vec![0u8; SPRITE_RGBA_BYTES];

    if offset == 0 || (offset as u64) >= file_len {
        return Ok(output);
    }

    reader.seek(SeekFrom::Start(offset as u64)).context("Failed to seek to sprite offset")?;

    // Skip 3 bytes chroma key (R, G, B - usually 0xFF, 0x00, 0xFF)
    let mut _chroma = [0u8; 3];
    reader.read_exact(&mut _chroma).context("Failed to read chroma key")?;

    let data_size = reader.read_u16::<LittleEndian>().context("Failed to read sprite data size")? as usize;
    if data_size == 0 {
        return Ok(output);
    }

    let start_pos = offset as u64 + 5; // 3 chroma + 2 data_size
    let mut bytes_read: usize = 0;
    let mut pixel_index: usize = 0;

    while bytes_read < data_size && pixel_index < SPRITE_PIXELS {
        let transparent_pixels = reader.read_u16::<LittleEndian>().context("Failed to read transparent pixel count")? as usize;
        let colored_pixels = reader.read_u16::<LittleEndian>().context("Failed to read colored pixel count")? as usize;
        bytes_read += 4;

        pixel_index = pixel_index.saturating_add(transparent_pixels);

        for _ in 0..colored_pixels {
            let red = reader.read_u8().context("Failed to read red channel")?;
            let green = reader.read_u8().context("Failed to read green channel")?;
            let blue = reader.read_u8().context("Failed to read blue channel")?;
            let alpha = if is_transparent {
                let a = reader.read_u8().context("Failed to read alpha channel")?;
                bytes_read += 4;
                a
            } else {
                bytes_read += 3;
                0xFF
            };

            if pixel_index < SPRITE_PIXELS {
                let base = pixel_index * 4;
                output[base] = red;
                output[base + 1] = green;
                output[base + 2] = blue;
                output[base + 3] = alpha;
            }
            pixel_index += 1;
        }
    }

    Ok(output)
}

/// Legacy compatibility wrapper — decodes from a byte slice (used by existing code paths)
pub fn decode_sprite_rle(file_bytes: &[u8], offset: u32, is_transparent: bool) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(file_bytes);
    decode_sprite_rle_from_reader(&mut cursor, offset, file_bytes.len() as u64, is_transparent)
}

/// Helper function to inspect basic SPR header info without loading all offsets
pub fn inspect_spr_header<P: AsRef<Path>>(path: P) -> Result<(u32, u32, bool)> {
    let path = path.as_ref();
    let mut file = File::open(path).context("Failed to open SPR file")?;
    let file_len = file.metadata()?.len();

    let signature = file.read_u32::<LittleEndian>()?;
    let count_u16 = file.read_u16::<LittleEndian>()? as u32;

    // Check if extended by comparing with file length
    let expected_u16_bytes = 6u64 + (count_u16 as u64 * 4);
    let mut is_extended = false;
    let mut final_count = count_u16;

    if file_len >= 8 {
        file.seek(SeekFrom::Start(4))?;
        let count_u32 = file.read_u32::<LittleEndian>()?;
        let expected_u32_bytes = 8u64 + (count_u32 as u64 * 4);

        if count_u32 > 65535 || (expected_u32_bytes <= file_len && expected_u16_bytes != file_len) {
            is_extended = true;
            final_count = count_u32;
        }
    }

    Ok((signature, final_count, is_extended))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sprite_rle() {
        let dummy = vec![0u8; 100];
        let res = decode_sprite_rle(&dummy, 0, false).unwrap();
        assert_eq!(res.len(), SPRITE_RGBA_BYTES);
        assert!(res.iter().all(|&b| b == 0));
    }
}
