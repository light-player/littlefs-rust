//! LittleFS filesystem implementation.

mod format;
mod mount;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;

pub struct LittleFs {
    _private: (),
}

impl Default for LittleFs {
    fn default() -> Self {
        Self::new()
    }
}

impl LittleFs {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn format<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        format::format(bd, config)
    }

    pub fn mount<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        mount::mount(bd, config)
    }

    pub fn unmount(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
