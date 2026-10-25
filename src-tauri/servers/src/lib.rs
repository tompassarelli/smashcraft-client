//! Stand-in for the private Smashcraft Servers crate, so a build with the
//! `smashcraft-servers` feature compiles without it. A build with the real
//! crate puts it in this folder.

#[derive(Default)]
pub struct Switch;

impl Switch {
    pub fn available(&self) -> Result<(), String> {
        Err("This copy of Smashcraft was built with a stand-in for Smashcraft Servers. Use Battle.net instead.".to_owned())
    }

    pub fn enable(&self, _warcraft_pid: u32) -> Result<(), String> {
        self.available()
    }

    pub fn disable(&self) {}
}
