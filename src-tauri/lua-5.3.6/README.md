# Lua 5.3.6 with 32-bit numbers

The Replays page plays a replay in the simulation of the map that recorded it:
that map's own war3map.lua, in Lua 5.3 built with `LUA_32BITS`, the number
model Warcraft III's Lua uses (smashcraft-client:src-tauri/src/mapsim.rs).
These are lua.org's 5.3.6 sources unchanged (lua-5.3.6.tar.gz, SHA-256
fc5fd69bb8736323f026672b1b7235da613d7177e72558893a0bdcd320466d60), without
the standalone interpreter and compiler; build.rs compiles them. MIT licensed:
COPYRIGHT.
