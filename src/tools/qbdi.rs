use crate::{Entry, Statistics};

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[derive(Default)]
struct Item {
    total: u32,
    mem_read: u32,
    mem_write: u32,
    stack_read: u32,
    stack_write: u32,
    call: u32,
    branch: u32,
    r#return: u32,
    compare: u32,
    logic: u32,
    arithmetic: u32,
    others: HashMap<String, u32>,
}

pub fn run_qbdi(
    request: crate::CommandRequest,
    options: &crate::ToolOptions,
) -> Result<crate::ToolOutput, ()> {
    let mut command = if cfg!(target_os = "windows") {
        let root = std::env::current_exe().unwrap();
        let mut command = {
            let preloader_name = "QBDIWinPreloader.exe";
            let preloader = root.parent().unwrap().join(preloader_name);
            if !preloader.is_file() {
                eprintln!(
                    "{preloader_name:?} not adjacent to {root}. {preloader} does not exist",
                    preloader = preloader.display(),
                    root = root.display(),
                );
                return Err(());
            }
            Command::new(preloader.display().to_string())
        };
        {
            let library = super::adjacent_qbdi_lib(true).unwrap();
            command.arg(library.display().to_string());
        }

        command.arg(request.program);
        command.args(request.arguments);
        command
    } else {
        let mut command = Command::new(request.program);

        #[cfg(target_os = "macos")]
        {
            let library = super::adjacent_qbdi_lib(true).unwrap();
            command.env("DYLD_BIND_AT_LAUNCH", "1");
            command.env("DYLD_INSERT_LIBRARIES", library.display().to_string());
        }

        #[cfg(target_os = "linux")]
        {
            let library = super::adjacent_qbdi_lib(true).unwrap();
            command.env("LD_BIND_NOW", "1");
            command.env("LD_PRELOAD", dbg!(library.display().to_string()));
        }

        command.args(request.arguments);
        command
    };

    command.stdout(Stdio::piped());

    let mut child = command.spawn().unwrap();

    let content = BufReader::new(child.stdout.take().unwrap());

    let mut total: Statistics = Statistics::default();
    let mut internal: Item = Item::default();

    // TODO this seems highly inefficient
    let mut items: HashMap<String, Item> = HashMap::new();

    for line in content.lines() {
        let line = line.unwrap();

        #[cfg(target_os = "linux")]
        eprintln!("TEMP linux: {line}");

        if let Some(rest) = line.strip_prefix("depict_qbdi::") {
            let Some((func, rest)) = rest.split_once('/') else {
                // TODO not sure why some items do not finish?
                continue;
            };

            let Some((kind, count)) = rest.split_once('/') else {
                dbg!(rest);
                continue;
            };
            let Ok(count) = count.parse() else {
                // TODO ...?
                dbg!(kind, count, rest);
                continue;
            };

            let func = format!("{func:#}", func = rustc_demangle::demangle(func));

            let item: &mut Item = if options.merge_internals {
                // TODO some other things are needed here
                let bad_prefixes = &[
                    "std::",
                    "core::",
                    "alloc::",
                    "_",
                    "*",
                    "OUTLINED_FUNCTION_",
                    // "<std::",
                ];
                let skip = bad_prefixes.iter().any(|prefix| func.starts_with(prefix));
                if skip {
                    &mut internal
                } else {
                    items.entry(func).or_default()
                }
            } else {
                items.entry(func).or_default()
            };

            total.total += count;
            item.total += count;

            match kind {
                "mem_read" => {
                    item.mem_read = count;
                    total.mem_read += count;
                }
                "mem_write" => {
                    item.mem_write = count;
                    total.mem_write += count;
                }
                "call" => {
                    item.call = count;
                    total.call += count;
                }
                "return" => {
                    item.r#return = count;
                    total.r#return += count;
                }
                "branch" => {
                    item.branch = count;
                    total.branch += count;
                }
                "compare" => {
                    item.compare = count;
                    total.compare += count;
                }
                "logic" => {
                    item.logic = count;
                    total.logic += count;
                }
                "arithmetic" => {
                    item.arithmetic = count;
                    total.arithmetic += count;
                }
                kind => {
                    item.others.insert(kind.to_owned(), count);
                    total.add_other(kind.to_owned(), count);
                }
            }
        } else {
            println!("{line}");
        }
    }

    child.wait().unwrap();

    if options.merge_internals {
        items.insert("Internal".to_owned(), internal);
    }

    let symbols: Vec<_> = items
        .into_iter()
        .map(|(name, item)| Entry {
            symbol_name: name,
            statistics: Statistics {
                total: item.total,
                mem_read: item.mem_read,
                mem_write: item.mem_write,
                stack_read: item.stack_read,
                stack_write: item.stack_write,
                call: item.call,
                r#return: item.r#return,
                branch: item.branch,
                compare: item.compare,
                logic: item.logic,
                arithmetic: item.arithmetic,
                others: item.others,
            },
        })
        .collect();

    Ok(crate::ToolOutput::SymbolInstructionCounts { total, symbols })
}
