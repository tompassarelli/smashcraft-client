//! Lua 5.3.6 built with 32-bit numbers (lua-5.3.6/README.md), behind the
//! smallest surface the client needs: run a chunk, call a global function
//! with text arguments and take its text result. Only text crosses, so the
//! number types the build chose never reach Rust.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

#[repr(C)]
struct LuaState {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn luaL_newstate() -> *mut LuaState;
    fn luaL_openlibs(state: *mut LuaState);
    fn luaL_loadbufferx(state: *mut LuaState, buffer: *const c_char, size: usize, name: *const c_char, mode: *const c_char) -> c_int;
    fn lua_pcallk(state: *mut LuaState, args: c_int, results: c_int, handler: c_int, context: isize, continuation: *const c_void) -> c_int;
    fn lua_settop(state: *mut LuaState, index: c_int);
    fn lua_getglobal(state: *mut LuaState, name: *const c_char) -> c_int;
    fn lua_pushlstring(state: *mut LuaState, text: *const c_char, length: usize) -> *const c_char;
    fn lua_tolstring(state: *mut LuaState, index: c_int, length: *mut usize) -> *const c_char;
    fn lua_close(state: *mut LuaState);
}

/// One Lua state.
pub struct Lua {
    state: *mut LuaState,
}

// A state is used by one thread at a time: the client keeps each behind a Mutex.
unsafe impl Send for Lua {}

impl Lua {
    pub fn new() -> Result<Self, String> {
        let state = unsafe { luaL_newstate() };
        if state.is_null() {
            return Err("Lua could not start".to_owned());
        }
        unsafe { luaL_openlibs(state) };
        Ok(Self { state })
    }

    /// The text at the top of the stack, then the stack emptied.
    fn take_text(&self) -> String {
        let mut length = 0usize;
        let text = unsafe { lua_tolstring(self.state, -1, &mut length) };
        let result = if text.is_null() {
            String::new()
        } else {
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(text.cast::<u8>(), length) }).into_owned()
        };
        unsafe { lua_settop(self.state, 0) };
        result
    }

    /// Runs `code` as a chunk named `name`.
    pub fn run(&self, code: &str, name: &str) -> Result<(), String> {
        let chunk = CString::new(format!("={name}")).map_err(|e| e.to_string())?;
        let mode = c"t";
        let loaded = unsafe { luaL_loadbufferx(self.state, code.as_ptr().cast(), code.len(), chunk.as_ptr(), mode.as_ptr()) };
        if loaded != 0 || unsafe { lua_pcallk(self.state, 0, 0, 0, 0, std::ptr::null()) } != 0 {
            return Err(self.take_text());
        }
        unsafe { lua_settop(self.state, 0) };
        Ok(())
    }

    /// Calls the global function `name` with text arguments; its result as text.
    pub fn call(&self, name: &CStr, args: &[&str]) -> Result<String, String> {
        unsafe { lua_getglobal(self.state, name.as_ptr()) };
        for arg in args {
            unsafe { lua_pushlstring(self.state, arg.as_ptr().cast(), arg.len()) };
        }
        let status = unsafe { lua_pcallk(self.state, args.len() as c_int, 1, 0, 0, std::ptr::null()) };
        let text = self.take_text();
        if status == 0 { Ok(text) } else { Err(text) }
    }
}

impl Drop for Lua {
    fn drop(&mut self) {
        unsafe { lua_close(self.state) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_32_bits_like_warcraft() {
        let lua = Lua::new().unwrap();
        lua.run("function probe(text) return tostring(2147483647 + tonumber(text)) .. ' ' .. tostring(16777217.0) .. ' ' .. math.type(1) end", "probe").unwrap();
        // Integers wrap at 32 bits; floats are binary32, so 2^24 + 1 rounds to 2^24.
        assert_eq!(lua.call(c"probe", &["1"]).unwrap(), "-2147483648 1.677722e+07 integer");
        assert!(lua.call(c"missing", &[]).unwrap_err().contains("attempt to call"));
        assert!(lua.run("this is not Lua", "broken").unwrap_err().contains("broken"));
    }
}
