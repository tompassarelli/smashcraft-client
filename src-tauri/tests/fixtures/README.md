`stormlib.w3x`: a 512-byte map header followed by an MPQ archive StormLib 9.30
created (format 1, sector shift 3): `war3map.lua` (3001 lines, zlib, many
sectors), `secret\note.txt` (zlib, encrypted with a fixed key) and `plain.txt`
(stored). src/mpq.rs reads it.
