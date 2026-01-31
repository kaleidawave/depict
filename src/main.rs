use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;

use depict::{CommandRequest, Entry, Statistics, ToolOptions, ToolOutput, tools, utilities};
use utilities::{Direction, PairedWriter, Sorting};

fn main() {
    let mut args = std::env::args().skip(1);
    let tool = args.next();
    let tool = tool.as_deref().unwrap_or("help");

    let input = BenchmarkInput::from_arguments(args);

    let writer = PairedWriter::new_from_option(
        input.write_to_stdout.then(std::io::stdout),
        input.write_results_to.map(|path| {
            let path = std::path::Path::new(&path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::File::create(path).unwrap()
        }),
    );

    if input.limit != usize::MAX && input.sort.is_none() {
        panic!("--limit requires --sort");
    }

    match tool {
        "--info" | "--help" | "help" => {
            println!("depict");
            println!("run 'qbdi', 'sde', 'perf-events' or 'time'");
        }
        "qbdi" => {
            let request = CommandRequest {
                program: input.program.into(),
                arguments: input.arguments.into_iter().map(Into::into).collect(),
            };
            let options = ToolOptions {
                keep: input.keep,
                merge_internals: input.merge_internals,
            };
            let result = tools::qbdi::run_qbdi(request, &options).unwrap();
            match result {
                ToolOutput::SymbolInstructionCounts { symbols, total } => print_results(
                    &mut writer.expect("--quiet must have --write-results-to"),
                    symbols,
                    total,
                    input.format,
                    input.sort,
                    input.limit,
                    input.breakdown,
                )
                .unwrap(),
                _ => todo!(),
            }
        }
        "time" => {
            todo!()
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64", debug_assertions))]
        "sde" => {
            let request = CommandRequest {
                program: input.program.into(),
                arguments: input.arguments.into_iter().map(Into::into).collect(),
            };
            let options = ToolOptions {
                keep: input.keep,
                merge_internals: input.merge_internals,
            };
            let result = tools::sde::run_sde(request, &options).unwrap();
            match result {
                ToolOutput::SymbolInstructionCounts { symbols, total } => print_results(
                    &mut writer.expect("--quiet must have --write-results-to"),
                    symbols,
                    total,
                    input.format,
                    input.sort,
                    input.limit,
                    input.breakdown,
                )
                .unwrap(),
                _ => todo!(),
            }
        }
        #[cfg(target_family = "unix")]
        "perf-events" => {
            todo!()
        }
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        "install-qbdi" => {
            use std::process::{Command, Stdio};

            let file = "qbdi.pkg";

            Command::new("curl")
                .arg("https://github.com/QBDI/QBDI/releases/download/v0.12.0/QBDI-0.12.0-osx-AARCH64.pkg")
                .arg("-L")
                .arg("-o")
                .arg(file)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            let dest = std::env::home_dir().unwrap().join("qbdi-out");
            std::fs::create_dir_all(&dest).unwrap();

            Command::new("sudo")
                .arg("installer")
                .arg("-pkg")
                .arg(file)
                .arg("-target")
                .arg(dest)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            std::fs::remove_file(file).unwrap();
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        "install-sde" => {
            // Based on https://github.com/petarpetrovt/setup-sde/blob/main/index.ts

            use std::process::{Command, Stdio};

            let base = "https://downloadmirror.intel.com";

            #[cfg(target_os = "linux")]
            let (platform, extension) = ("lin", "");

            #[cfg(target_os = "macos")]
            let (platform, extension) = ("mac", "");

            #[cfg(target_os = "windows")]
            let (platform, extension) = ("win", ".exe");

            let url = format!("{base}/859732/sde-external-9.58.0-2025-06-16-{platform}.tar.xz");
            let file = "sde-temp-file.tar.xz";

            Command::new("curl")
                .arg(url)
                .arg("-o")
                .arg(file)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            // exec.exec(`"${tarExePath}"`, [`x`, `--force-local`, `-C`, `${extractedFilesPath}`, `-f`, `${tarFilePath}`]);

            dbg!(file);

            Command::new("tar")
                // -x extract, -v verbose, -j archive with gzip/bzip2/xz/lzma, -f pass filename
                .arg("-xvf") // -xvjf
                .arg(file)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            let dest = depict::adjacent_sde_path().unwrap();
            let target = format!("sde-external-9.58.0-2025-06-16-{platform}/sde{extension}");
            std::fs::rename(target, dest).unwrap();
        }
        tool => {
            println!("unknown tool {tool:?}. run with 'qbdi', 'sde', 'perf-events' or 'time'");
        }
    }
}

#[derive(Debug, Default)]
pub enum OutputFormat {
    #[default]
    Plain,
    JSON,
    CSV,
    Markdown,
}

#[derive(Debug)]
pub struct BenchmarkInput {
    /// number of symbol entries to show
    pub limit: usize,
    pub sort: Option<Sorting>,
    /// plain, JSON, markdown, csv
    pub format: OutputFormat,
    // ...
    pub program: OsString,
    pub arguments: Vec<OsString>,
    // ...
    pub generic_arguments: HashMap<String, Vec<String>>,

    // TODO
    /// Save SDE file...
    pub keep: Option<String>,
    /// skip Rust internals
    pub merge_internals: bool,
    /// include all instruction kinds
    pub breakdown: bool,
    // things
    pub write_results_to: Option<String>,
    pub write_to_stdout: bool,
}

impl BenchmarkInput {
    pub fn from_arguments(mut args: impl Iterator<Item = String>) -> Self {
        let mut this = Self {
            limit: usize::MAX,
            sort: Some(Sorting {
                field: "total".into(),
                direction: Direction::Ascending,
            }),
            format: OutputFormat::default(),
            // ...
            program: OsString::new(),
            arguments: Vec::new(),
            // ...
            generic_arguments: HashMap::new(),
            // ...
            keep: None,
            merge_internals: false,
            breakdown: false,
            // ...
            write_results_to: None,
            write_to_stdout: true,
        };

        let mut left_over: Option<String> = None;
        while let Some(arg) = left_over.take().or_else(|| args.next()) {
            match arg.as_str() {
                "--format" => {
                    let format = args.next().expect("no format given");
                    this.format = match format.as_str() {
                        "plain" => OutputFormat::Plain,
                        "json" => OutputFormat::JSON,
                        "markdown" => OutputFormat::Markdown,
                        format => {
                            eprintln!("Unknown output format '{format:?}'");
                            OutputFormat::Plain
                        }
                    };
                }
                "--sort" => {
                    let field = args.next().expect("expected field");
                    let next = args.next();
                    let direction = match next.as_deref() {
                        Some("asc" | "ascending") => Direction::Ascending,
                        Some("desc" | "descending") => Direction::Descending,
                        _ => {
                            left_over = next;
                            Direction::Ascending
                        }
                    };
                    this.sort = Some(Sorting { field, direction });
                }
                // "--blocks" => {
                //     blocks = args.next().unwrap().parse().expect("invalid top blocks");
                // }
                "--limit" => {
                    let limit = args.next().unwrap();
                    if "all" == limit {
                        this.limit = usize::MAX;
                    } else {
                        this.limit = limit.parse().expect("invalid limit");
                    }
                }
                "--keep" => {
                    this.keep = args.next();
                }
                "--write-results-to" => {
                    this.write_results_to = args.next();
                }
                "--merge-internals" => {
                    this.merge_internals = true;
                }
                "--breakdown" => {
                    this.breakdown = true;
                }
                "--quiet" => {
                    this.write_to_stdout = false;
                }
                "--arg" => {
                    // `--arg name=6,7`
                    let next = args.next().unwrap();
                    let (name, values) = next.split_once('=').unwrap();
                    // TODO CSV parse?
                    let values = values.trim().split(',').map(str::to_owned).collect();
                    this.generic_arguments.insert(name.to_owned(), values);
                }
                // -- *program* *arg1* *arg2* ...
                "--" => {
                    let arg = args.next().unwrap();
                    this.program = OsString::from(arg);
                    break;
                }
                // WIP. First unknown argument is the program
                _command => {
                    this.program = OsString::from(arg);
                    break;
                }
            }
        }

        this.arguments = args.map(OsString::from).collect();

        this
    }
}

pub fn print_results(
    to: &mut impl Write,
    mut rows: Vec<Entry>,
    total: Statistics,
    output_format: OutputFormat,
    sorting: Option<utilities::Sorting>,
    limit: usize,
    breakdown: bool,
) -> std::io::Result<()> {
    use std::borrow::Cow;
    use utilities::count_with_seperator;

    const MAX_WIDTH: usize = 100;
    const WHITESPACE: &str = if let Ok(result) = str::from_utf8(&[b' '; MAX_WIDTH]) {
        result
    } else {
        ""
    };

    if let Some(ref sort) = sorting {
        match sort.field.as_str() {
            "name" => {
                rows.sort_unstable_by(|lhs, rhs| {
                    sort.direction.compare(&lhs.symbol_name, &rhs.symbol_name)
                });
            }
            "total" => {
                rows.sort_unstable_by(|lhs, rhs| {
                    sort.direction
                        .compare(&lhs.statistics.total, &rhs.statistics.total)
                });
            }
            field => {
                writeln!(to, "error: unknown field {field:?}")?;
            }
        }
    }

    let skip = if let Some(utilities::Sorting {
        direction: utilities::Direction::Descending,
        ..
    }) = sorting
    {
        rows.len().saturating_sub(limit)
    } else {
        0
    };

    rows.insert(
        0,
        Entry {
            symbol_name: "Total".into(),
            statistics: total,
        },
    );

    let rows = &rows[skip..];
    let rows = &rows[..std::cmp::min(rows.len(), limit)];

    match output_format {
        OutputFormat::Plain => {
            let max_name_width = {
                let mut max_name_width = 0;
                for row in rows {
                    max_name_width = std::cmp::max(max_name_width, row.symbol_name.len());
                }
                std::cmp::min(max_name_width, MAX_WIDTH)
            };

            // for (func, mut item) in rows {
            //     print!("{func} - {total} instructions", total = item.total);
            //     item.instruction_kind
            //         .sort_unstable_by_key(|(_, value)| u32::MAX - value);
            //     for (kind, count) in &item.instruction_kind {
            //         print!(" ({kind}={count})");
            //     }
            //     println!();
            // }

            for row in rows {
                let symbol_name: Cow<'_, str> = if row.symbol_name.len() > MAX_WIDTH {
                    Cow::Owned(format!(
                        "{prefix}...",
                        prefix = &row.symbol_name[..MAX_WIDTH - 3]
                    ))
                } else {
                    Cow::Borrowed(&row.symbol_name)
                };
                let fill = &WHITESPACE[..max_name_width - symbol_name.len()];

                write!(to, "{symbol_name}{fill}")?;
                write!(
                    to,
                    " total: {count}",
                    count = count_with_seperator(row.statistics.total as usize)
                )?;
                if breakdown {
                    write!(
                        to,
                        " mem_read: {mem_read}, mem_write: {mem_write}, stack_read: {stack_read}, stack_write: {stack_write}, call {call}",
                        mem_read = count_with_seperator(row.statistics.mem_read as usize),
                        mem_write = count_with_seperator(row.statistics.mem_write as usize),
                        stack_read = count_with_seperator(row.statistics.stack_read as usize),
                        stack_write = count_with_seperator(row.statistics.stack_write as usize),
                        call = count_with_seperator(row.statistics.call as usize)
                    )?;
                    // TODO
                    // for (name, count) in &row.statistics.others {
                    //     write!(
                    //         to,
                    //         " {name}: {count}",
                    //         count = count_with_seperator(*count as usize)
                    //     )?;
                    // }
                }
                writeln!(to)?;
            }

            Ok(())
        }
        OutputFormat::JSON => {
            let mut buf = String::from("[");
            for row in rows {
                if buf.len() > 1 {
                    buf.push(',');
                }
                if breakdown {
                    buf.push_str(&json_builder_macro::json! {
                        symbol_name: row.symbol_name.as_str(),
                        total: row.statistics.total,
                        mem_read: row.statistics.mem_read,
                        mem_write: row.statistics.mem_write,
                        stack_read: row.statistics.stack_read,
                        stack_write: row.statistics.stack_write,
                        call: row.statistics.call,
                        kinds: row.statistics.others
                    });
                } else {
                    buf.push_str(&json_builder_macro::json! {
                        symbol_name: row.symbol_name.as_str(),
                        total: row.statistics.total
                    });
                }
            }
            buf.push(']');
            write!(to, "{buf}")
        }
        OutputFormat::CSV => {
            if breakdown {
                writeln!(
                    to,
                    "symbol name,count,mem_read,mem_write,stack_read,stack_write,call"
                )?;
            } else {
                writeln!(to, "symbol name,count")?;
            }
            for row in rows {
                let Entry {
                    symbol_name,
                    statistics,
                } = row;
                if breakdown {
                    let Statistics {
                        total,
                        mem_read,
                        mem_write,
                        stack_read,
                        stack_write,
                        call,
                        others: _,
                    } = statistics;
                    writeln!(
                        to,
                        "`{symbol_name}`,{total},{mem_read},{mem_write},{stack_read},{stack_write},{call}"
                    )?;
                    // TODO ..?
                    // for (name, count) in &row.others {
                    //     write!(
                    //         to,
                    //         ",\"{name}\",{count}",
                    //         count = *count as usize
                    //     )?;
                    // }
                } else {
                    writeln!(to, "|`{symbol_name}`|{total}|", total = statistics.total)?;
                }
            }
            Ok(())
        }
        OutputFormat::Markdown => {
            if breakdown {
                writeln!(
                    to,
                    "|symbol name|count|mem_read|mem_write|stack_read|stack_write|call|other|"
                )?;
                writeln!(to, "|---|---|---|---|---|---|---|")?;
            } else {
                writeln!(to, "|symbol name|count|")?;
                writeln!(to, "|---|---|")?;
            }
            for row in rows {
                let Entry {
                    symbol_name,
                    statistics,
                } = row;
                if breakdown {
                    let Statistics {
                        total,
                        mem_read,
                        mem_write,
                        stack_read,
                        stack_write,
                        call,
                        others: _,
                    } = statistics;
                    writeln!(
                        to,
                        "|`{symbol_name}`|{total}|{mem_read}|{mem_write}|{stack_read}|{stack_write}|{call}|"
                    )?;
                    // TODO ..?
                    // for (name, count) in &row.others {
                    //     write!(
                    //         to,
                    //         "|\"{name}\"|{count}",
                    //         count = *count as usize
                    //     )?;
                    // }
                } else {
                    writeln!(to, "|`{symbol_name}`|{total}|", total = statistics.total)?;
                }
            }
            Ok(())
        }
    }
}
