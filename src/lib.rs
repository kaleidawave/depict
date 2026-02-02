pub mod tools;
pub mod utilities;

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

pub struct CommandRequest<'a> {
    pub program: Cow<'a, OsStr>,
    pub arguments: Vec<Cow<'a, OsStr>>,
}

/// TODO iterations, etc for time etc
pub struct ToolOptions {
    /// Save SDE file...
    pub keep: Option<String>,
    /// skip Rust internals
    pub merge_internals: bool,
}

#[non_exhaustive]
pub enum ToolOutput {
    SymbolInstructionCounts {
        symbols: Vec<Entry>,
        total: Statistics,
    },
}

/// TODO there more be more: branch, compare, return, etc
#[derive(Clone, Debug)]
pub struct Entry {
    pub symbol_name: String,
    pub statistics: Statistics,
}

/// Some copied from sde-output-parser
#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub total: u32,
    pub mem_read: u32,
    pub mem_write: u32,
    pub stack_read: u32,
    pub stack_write: u32,
    pub call: u32,
    pub others: HashMap<String, u32>,
}

impl std::ops::AddAssign for Statistics {
    fn add_assign(&mut self, rhs: Self) {
        let Self {
            total,
            mem_read,
            mem_write,
            stack_read,
            stack_write,
            call,
            others,
        } = rhs;
        self.total += total;
        self.mem_read += mem_read;
        self.mem_write += mem_write;
        self.stack_read += stack_read;
        self.stack_write += stack_write;
        self.call += call;
        for (key, value) in others.into_iter() {
            self.add_other(key, value);
        }
    }
}

impl Statistics {
    pub fn add_other(&mut self, key: String, value: u32) {
        self.total += value;
        if let Some(existing) = self.others.get_mut(&key) {
            *existing += value;
        } else {
            self.others.insert(key, value);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn adjacent_sde_path() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(path.parent()?.join("sde").join("sde").with_extension("exe"))
}

#[cfg(unix)]
pub fn adjacent_sde_path() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(path.parent()?.join("sde").join("sde"))
}
