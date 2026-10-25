//! The facts a relay room carries about its map (wisp:host/lan/map.ts,
//! `readMapFacts`): its path in the game, size, CRC-32 and SHA-1 of the whole
//! file, the checksum over its script and data files, and the size, layout,
//! players and forces of its war3map.w3i.

use relay_proto::room::{MapFacts, MapPlayer};
use sha1::{Digest, Sha1};
use std::path::Path;

use crate::mpq;

const FORMAT: i32 = 39;
const USE_CUSTOM_FORCES: i32 = 0x40;
const FIXED_PLAYER_SETTINGS: i32 = 0x20;
const SCRIPT_ENTRIES: [&str; 4] = ["war3map.j", "scripts\\war3map.j", "war3map.lua", "scripts\\war3map.lua"];
const CHECKED_ENTRIES: [&str; 9] = ["war3map.w3e", "war3map.wpm", "war3map.doo", "war3map.w3u", "war3map.w3b", "war3map.w3d", "war3map.w3a", "war3map.w3q", "war3map.w3l"];
const ENVIRONMENT: [(u8, usize); 14] = [(1, 1), (4, 2), (0, 4), (4, 1), (0, 4), (4, 7), (4, 4), (4, 1), (0, 1), (1, 1), (4, 1), (4, 3), (4, 3), (4, 10)];

pub fn sha1(file: &[u8]) -> [u8; 20] {
    Sha1::digest(file).into()
}

/// `Maps\...` from the last Maps folder in the map's path, as Warcraft III names it.
pub fn path_in_game(map: &Path) -> Result<String, String> {
    let parts: Vec<String> = map.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let maps = parts.iter().rposition(|p| p.eq_ignore_ascii_case("maps")).ok_or(format!("{} is not under a Maps folder", map.display()))?;
    Ok(std::iter::once("Maps").chain(parts[maps + 1..].iter().map(String::as_str)).collect::<Vec<_>>().join("\\"))
}

pub fn read(map: &Path) -> Result<MapFacts, String> {
    let file = std::fs::read(map).map_err(|e| format!("{}: {e}", map.display()))?;
    facts(&file, &path_in_game(map)?)
}

fn xoro_update(start: u32, bytes: &[u8]) -> u32 {
    let mut value = start;
    let mut words = bytes.chunks_exact(4);
    for word in words.by_ref() {
        value = (value ^ u32::from_le_bytes(word.try_into().unwrap())).rotate_left(3);
    }
    for &byte in words.remainder() {
        value = (value ^ u32::from(byte)).rotate_left(3);
    }
    value
}

fn xoro(file: &[u8]) -> Result<u32, String> {
    let script = SCRIPT_ENTRIES.iter().find_map(|name| mpq::read_file(file, name).transpose()).ok_or("the map has no war3map.j or war3map.lua")??;
    let mut value = xoro_update(0, &script);
    for name in CHECKED_ENTRIES {
        if let Some(bytes) = mpq::read_file(file, name)? {
            value = xoro_update(value, &xoro_update(0, &bytes).to_le_bytes());
        }
    }
    Ok(value)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let slice = self.bytes.get(self.at..self.at + n).ok_or("truncated war3map.w3i")?;
        self.at += n;
        Ok(slice)
    }

    fn int(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn string(&mut self) -> Result<(), String> {
        let end = self.bytes[self.at..].iter().position(|&b| b == 0).ok_or("unterminated string in war3map.w3i")?;
        self.at += end + 1;
        Ok(())
    }
}

/// The facts of map `file` at `path`; an entry they cover that the archive holds but the client can't decode refuses the map.
fn facts(file: &[u8], path: &str) -> Result<MapFacts, String> {
    let w3i = mpq::read_file(file, "war3map.w3i")?.ok_or("the map has no war3map.w3i")?;
    let mut info = Cursor { bytes: &w3i, at: 0 };
    let format = info.int()?;
    if format != FORMAT {
        return Err(format!("war3map.w3i format {format} is not supported; expected {FORMAT}"));
    }
    info.take(24)?;
    for _ in 0..4 {
        info.string()?;
    }
    let bounds = info.take(56)?;
    let (width, height) = (i32::from_le_bytes(bounds[48..52].try_into().unwrap()), i32::from_le_bytes(bounds[52..56].try_into().unwrap()));
    let flags = info.int()?;
    for (size, count) in ENVIRONMENT {
        for _ in 0..count {
            if size == 0 { info.string()? } else { info.take(size.into())?; }
        }
    }
    let mut players = Vec::new();
    for _ in 0..info.int()? {
        let (id, controller, race) = (info.int()?, info.int()?, info.int()?);
        info.take(8)?;
        info.string()?;
        info.take(24)?;
        players.push(MapPlayer { id: id as u8, controller: controller as u8, race: race as u8 });
    }
    let mut forces = Vec::new();
    for _ in 0..info.int()? {
        info.take(4)?;
        forces.push(info.int()? as u32);
        info.string()?;
    }
    Ok(MapFacts {
        path: path.as_bytes().to_vec(),
        size: file.len() as u32,
        crc32: crc32fast::hash(file),
        sha1: sha1(file),
        xoro: xoro(file)?,
        width: width as u16,
        height: height as u16,
        layout: u8::from(flags & USE_CUSTOM_FORCES != 0) | (u8::from(flags & FIXED_PLAYER_SETTINGS != 0) << 1),
        forces,
        players,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpq::build::archive;

    fn w3i() -> Vec<u8> {
        let mut bytes = [FORMAT.to_le_bytes().as_slice(), &[0; 24], &[0; 4], &[0; 56], &0i32.to_le_bytes()].concat();
        for (size, count) in ENVIRONMENT {
            bytes.extend(vec![0; usize::from(size.max(1)) * count]);
        }
        bytes.extend([0; 8]);
        bytes
    }

    #[test]
    fn a_map_with_an_entry_the_client_cant_decode_is_refused_naming_it_instead_of_hosted_with_a_wrong_checksum() {
        let (info, script, terrain) = (w3i(), b"-- war3map.lua".as_slice(), b"terrain!".as_slice());
        let readable = archive(&[("war3map.w3i", &info, info.len()), ("war3map.lua", script, script.len()), ("war3map.w3e", terrain, terrain.len())]);
        let expected = xoro_update(xoro_update(0, script), &xoro_update(0, terrain).to_le_bytes());
        assert_eq!(facts(&readable, "Maps\\test.w3x").unwrap().xoro, expected);
        let bzip2 = [0x10, 1, 2, 3].as_slice();
        for broken in ["war3map.w3i", "war3map.lua", "war3map.w3e"] {
            let entries: Vec<(&str, &[u8], usize)> = [("war3map.w3i", info.as_slice()), ("war3map.lua", script), ("war3map.w3e", terrain)]
                .into_iter()
                .map(|(name, bytes)| if name == broken { (name, bzip2, 64) } else { (name, bytes, bytes.len()) })
                .collect();
            let unreadable = archive(&[entries.as_slice(), &[("scripts\\war3map.lua", script, script.len())]].concat());
            assert_eq!(facts(&unreadable, "Maps\\test.w3x").unwrap_err(), format!("{broken}: a sector uses compression 0x10, which the client doesn't read"));
        }
    }
}
