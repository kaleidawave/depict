pub mod tools;
pub mod utilities;

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;

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
    pub branch: u32,
    pub r#return: u32,
    pub compare: u32,
    pub logic: u32,
    pub arithmetic: u32,
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
            branch,
            r#return,
            compare,
            arithmetic,
            logic,
            others,
        } = rhs;
        self.total += total;
        self.mem_read += mem_read;
        self.mem_write += mem_write;
        self.stack_read += stack_read;
        self.stack_write += stack_write;
        self.call += call;
        self.branch += branch;
        self.r#return += r#return;
        self.compare += compare;
        self.logic += logic;
        self.arithmetic += arithmetic;
        for (key, value) in others {
            self.add_other(key, value);
        }
    }
}

impl Statistics {
    #[must_use]
    pub fn as_rows(&self) -> [(&str, u32); 12] {
        let Self {
            total,
            mem_read,
            mem_write,
            stack_read,
            stack_write,
            call,
            branch,
            r#return,
            compare,
            arithmetic,
            logic,
            others,
        } = self;
        [
            ("total", *total),
            ("mem_read", *mem_read),
            ("mem_write", *mem_write),
            ("stack_read", *stack_read),
            ("stack_write", *stack_write),
            ("call", *call),
            ("branch", *branch),
            ("return", *r#return),
            ("compare", *compare),
            ("logic", *logic),
            ("arithmetic", *arithmetic),
            ("other", others.values().copied().sum()),
        ]
    }

    pub fn add_other(&mut self, key: String, value: u32) {
        // DO NOT DO THIS AS DOUBLE COUNTING self.total += value;
        if let Some(existing) = self.others.get_mut(&key) {
            *existing += value;
        } else {
            self.others.insert(key, value);
        }
    }
}
