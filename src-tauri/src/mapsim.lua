-- Loads a Smashcraft map's bundle without starting the map, adds the replay
-- viewer's modules and calls its driver; src-tauri/src/mapsim.rs
-- runs it in 32-bit Lua. Plain Lua: it runs before any of the map's code.
local requireModule, modules, driver
function smashcraft_load_map(war3)
  local open = war3:find("assert(load([=[", 1, true)
  local close = open and war3:find("]=],", open, true)
  if not close then error("the map holds no Smashcraft bundle") end
  local bundle = war3:sub(open + #"assert(load([=[", close - 1)
  local entry = bundle:find("\nlocal ____entry = require(", 1, true)
  if not entry then error("the map's bundle has no entry") end
  requireModule, modules = assert(load(bundle:sub(1, entry) .. "return require, ____modules\n", "=map"))()
  return ""
end
function smashcraft_add_viewer(text)
  local env = setmetatable({ require = requireModule }, { __index = _G })
  for name, module in pairs(assert(load(text, "=viewer", "t", env))()) do modules[name] = module end
  driver = requireModule("game.replay.viewerDriver")
  return ""
end
function smashcraft_open(text) return driver.open(text) end
function smashcraft_advance(frames) return driver.advance(math.tointeger(tonumber(frames)) or 0) end
function smashcraft_seek(frame) return driver.seek(math.tointeger(tonumber(frame)) or 0) end
