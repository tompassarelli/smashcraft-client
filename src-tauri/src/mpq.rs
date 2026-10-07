//! Reads one file from a Warcraft III map, an MPQ archive (format 1, the one
//! Warcraft III and StormLib's map builds use): the hash and block tables,
//! encrypted names and keys, sectors and zlib compression. The client reads
//! war3map.lua to play a replay in that map's simulation (mapsim.rs).

use std::io::{Read, Seek, SeekFrom};

const MAGIC: &[u8; 4] = b"MPQ\x1a";
const COMPRESSED: u32 = 0x0000_0200;
const IMPLODED: u32 = 0x0000_0100;
const ENCRYPTED: u32 = 0x0001_0000;
const FIX_KEY: u32 = 0x0002_0000;
const SINGLE_UNIT: u32 = 0x0100_0000;
const SECTOR_CRC: u32 = 0x0400_0000;
const EXISTS: u32 = 0x8000_0000;
const EMPTY: u32 = 0xFFFF_FFFF;
const ZLIB: u8 = 0x02;

fn crypt_table() -> &'static [u32; 0x500] {
    static TABLE: std::sync::OnceLock<[u32; 0x500]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; 0x500];
        let mut seed: u32 = 0x0010_0001;
        for index1 in 0..0x100 {
            let mut index2 = index1;
            for _ in 0..5 {
                seed = (seed * 125 + 3) % 0x2A_AAAB;
                let high = (seed & 0xFFFF) << 16;
                seed = (seed * 125 + 3) % 0x2A_AAAB;
                table[index2] = high | (seed & 0xFFFF);
                index2 += 0x100;
            }
        }
        table
    })
}

/// Storm's string hash of `name` (upper case, `/` as `\`) for hash `kind`: 0 table index, 1 and 2 the name checks, 3 a key.
fn hash(name: &str, kind: u32) -> u32 {
    let table = crypt_table();
    let (mut seed1, mut seed2) = (0x7FED_7FEDu32, 0xEEEE_EEEEu32);
    for byte in name.bytes() {
        let ch = match byte.to_ascii_uppercase() {
            b'/' => b'\\',
            other => other,
        } as u32;
        seed1 = table[(kind * 0x100 + ch) as usize] ^ seed1.wrapping_add(seed2);
        seed2 = ch.wrapping_add(seed1).wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3);
    }
    seed1
}

fn decrypt(words: &mut [u32], mut key: u32) {
    let table = crypt_table();
    let mut seed = 0xEEEE_EEEEu32;
    for word in words.iter_mut() {
        seed = seed.wrapping_add(table[(0x400 + (key & 0xFF)) as usize]);
        let plain = *word ^ key.wrapping_add(seed);
        key = ((!key << 0x15).wrapping_add(0x1111_1111)) | (key >> 0x0B);
        seed = plain.wrapping_add(seed).wrapping_add(seed << 5).wrapping_add(3);
        *word = plain;
    }
}

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Decrypts whole words of `bytes` in place; trailing bytes stay as they are.
fn decrypt_bytes(bytes: &mut [u8], key: u32) {
    let mut values = words(bytes);
    decrypt(&mut values, key);
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
}

/** `len` bytes at `at`; a map is read in parts, so finding one file in a large map reads little of it. */
fn read_at<R: Read + Seek>(source: &mut R, at: usize, len: usize) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; len];
    source.seek(SeekFrom::Start(at as u64)).map_err(|e| e.to_string())?;
    source.read_exact(&mut out).map_err(|_| "the archive is cut short".to_owned())?;
    Ok(out)
}

fn u32_at<R: Read + Seek>(source: &mut R, at: usize) -> Result<u32, String> {
    read_at(source, at, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn slice(bytes: &[u8], start: usize, len: usize) -> Result<&[u8], String> {
    bytes.get(start..start.checked_add(len).ok_or("the archive is malformed")?).ok_or_else(|| "the archive is cut short".to_owned())
}

/// A table of `entries` 16-byte entries at `offset`, decrypted with the key of `name`.
fn table<R: Read + Seek>(source: &mut R, archive: usize, offset: u32, entries: u32, name: &str) -> Result<Vec<[u32; 4]>, String> {
    if entries > 1 << 20 {
        return Err("the archive's tables are malformed".to_owned());
    }
    let raw = read_at(source, archive + offset as usize, entries as usize * 16)?;
    let mut values = words(&raw);
    decrypt(&mut values, hash(name, 3));
    Ok(values.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect())
}

fn decompress(sector: &[u8], size: usize) -> Result<Vec<u8>, String> {
    match sector.first() {
        Some(&ZLIB) => {
            let mut out = Vec::with_capacity(size);
            flate2::read::ZlibDecoder::new(&sector[1..]).read_to_end(&mut out).map_err(|e| format!("a sector doesn't inflate: {e}"))?;
            Ok(out)
        }
        Some(mask) => Err(format!("a sector uses compression {mask:#04x}, which the client doesn't read")),
        None => Err("an empty sector".to_owned()),
    }
}

/// The file `name` in the MPQ archive in `bytes`; the archive may follow a map header.
pub fn read_file(bytes: &[u8], name: &str) -> Result<Vec<u8>, String> {
    read_file_from(&mut std::io::Cursor::new(bytes), name)
}

/// The file `name` in the MPQ archive `source` holds, reading only the parts it needs.
pub fn read_file_from<R: Read + Seek>(source: &mut R, name: &str) -> Result<Vec<u8>, String> {
    let length = source.seek(SeekFrom::End(0)).map_err(|e| e.to_string())? as usize;
    let archive = (0..length).step_by(512).find(|&at| read_at(source, at, 4).is_ok_and(|magic| magic == MAGIC)).ok_or("not an MPQ archive")?;
    let shift = read_at(source, archive + 14, 2).map(|b| u16::from_le_bytes([b[0], b[1]]))?;
    let sector_size = 512usize << shift.min(20);
    let (hash_offset, block_offset) = (u32_at(source, archive + 16)?, u32_at(source, archive + 20)?);
    let (hash_entries, block_entries) = (u32_at(source, archive + 24)?, u32_at(source, archive + 28)?);
    if hash_entries == 0 {
        return Err("the archive has no files".to_owned());
    }
    let hashes = table(source, archive, hash_offset, hash_entries, "(hash table)")?;
    let blocks = table(source, archive, block_offset, block_entries, "(block table)")?;
    let (name_a, name_b) = (hash(name, 1), hash(name, 2));
    let start = hash(name, 0) % hash_entries;
    let mut block = None;
    for probe in 0..hash_entries {
        let [a, b, _, index] = hashes[((start + probe) % hash_entries) as usize];
        if index == EMPTY {
            break;
        }
        if a == name_a && b == name_b && (index as usize) < blocks.len() {
            block = Some(blocks[index as usize]);
            break;
        }
    }
    let [offset, packed, size, flags] = block.ok_or_else(|| format!("{name} isn't in the archive"))?;
    if flags & EXISTS == 0 {
        return Err(format!("{name} was deleted from the archive"));
    }
    if flags & IMPLODED != 0 {
        return Err(format!("{name} is imploded, which the client doesn't read"));
    }
    let owned = read_at(source, archive + offset as usize, packed as usize)?;
    let data = owned.as_slice();
    let size = size as usize;
    let mut key = 0;
    if flags & ENCRYPTED != 0 {
        let base = name.rsplit(['\\', '/']).next().unwrap_or(name);
        key = hash(base, 3);
        if flags & FIX_KEY != 0 {
            key = key.wrapping_add(offset) ^ size as u32;
        }
    }
    let compressed = flags & COMPRESSED != 0;
    if flags & SINGLE_UNIT != 0 {
        let mut unit = data.to_vec();
        if key != 0 {
            decrypt_bytes(&mut unit, key);
        }
        return if compressed && unit.len() < size { decompress(&unit, size) } else { Ok(unit) };
    }
    let sectors = size.div_ceil(sector_size);
    let bounds: Vec<usize> = if compressed {
        let count = sectors + 1 + usize::from(flags & SECTOR_CRC != 0);
        let mut table = slice(data, 0, count * 4)?.to_vec();
        if key != 0 {
            decrypt_bytes(&mut table, key.wrapping_sub(1));
        }
        words(&table).into_iter().take(sectors + 1).map(|v| v as usize).collect()
    } else {
        (0..=sectors).map(|i| (i * sector_size).min(packed as usize)).collect()
    };
    if bounds.windows(2).any(|w| w[0] > w[1]) || bounds.last().is_some_and(|&end| end > data.len()) {
        return Err(format!("{name}'s sector table is malformed"));
    }
    let mut out = Vec::with_capacity(size);
    for (index, window) in bounds.windows(2).enumerate() {
        let expected = (size - index * sector_size).min(sector_size);
        let mut sector = data[window[0]..window[1]].to_vec();
        if key != 0 {
            decrypt_bytes(&mut sector, key.wrapping_add(index as u32));
        }
        if compressed && sector.len() < expected {
            out.extend(decompress(&sector, expected)?);
        } else {
            out.extend_from_slice(&sector);
        }
    }
    out.truncate(size);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture() -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stormlib.w3x")).unwrap()
    }

    #[test]
    fn storm_hashes_match_known_values() {
        // Known values from StormLib's hash of "(hash table)" and "(block table)" keys.
        assert_eq!(hash("(hash table)", 3), 0xC3AF_3770);
        assert_eq!(hash("(block table)", 3), 0xEC83_B3A3);
    }

    #[test]
    fn reads_compressed_multi_sector_encrypted_and_plain_files_stormlib_wrote() {
        let map = fixture();
        let lua = String::from_utf8(read_file(&map, "war3map.lua").unwrap()).unwrap();
        assert!(lua.starts_with("-- fixture war3map.lua"));
        assert_eq!(lua.lines().count(), 3001);
        assert_eq!(read_file(&map, "secret\\note.txt").unwrap(), b"an encrypted file with a fixed key\n");
        assert_eq!(read_file(&map, "plain.txt").unwrap(), b"stored as it is\n");
        assert!(read_file(&map, "missing.txt").unwrap_err().contains("isn't in the archive"));
        assert_eq!(read_file(b"not a map", "war3map.lua").unwrap_err(), "not an MPQ archive");
    }
}
