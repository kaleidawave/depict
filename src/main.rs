use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;

use depict::{CommandRequest, Entry, Statistics, ToolOptions, ToolOutput, tools, utilities};
use utilities::{Direction, Sorting};

fn main() {
    let mut args = std::env::args().skip(1);
    let tool = args.next();
    let tool = tool.as_deref().unwrap_or("help");

    match tool {
        "--info" | "--help" | "help" => {
            println!("depict");
            println!("run 'count', 'install'"); // , 'perf-events' or 'time'
        }
        "time" => {
            todo!()
        }
        "qbdi" => {
            let input = BenchmarkInput::from_arguments(args);

            if input.limit != usize::MAX && input.sort.is_none() {
                panic!("--limit requires --sort");
            }
            let request = CommandRequest {
                program: input.program.into(),
                arguments: input.arguments.into_iter().map(Into::into).collect(),
            };
            let options = ToolOptions {
                keep: input.keep,
                merge_internals: input.merge_internals,
            };
            let result = tools::qbdi::run_qbdi(request, &options).unwrap();
            output_result(
                result,
                input.sort,
                input.limit,
                input.breakdown,
                input.write_results_to,
            );
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64", debug_assertions))]
        "sde" => {
            let input = BenchmarkInput::from_arguments(args);

            if input.limit != usize::MAX && input.sort.is_none() {
                panic!("--limit requires --sort");
            }
            let request = CommandRequest {
                program: input.program.into(),
                arguments: input.arguments.into_iter().map(Into::into).collect(),
            };
            let options = ToolOptions {
                keep: input.keep,
                merge_internals: input.merge_internals,
            };
            let result = tools::sde::run_sde(request, &options).unwrap();
            output_result(
                result,
                input.sort,
                input.limit,
                input.breakdown,
                input.write_results_to,
            );
        }
        "count" => {
            let input = BenchmarkInput::from_arguments(args);

            if input.limit != usize::MAX && input.sort.is_none() {
                panic!("--limit requires --sort");
            }
            let request = CommandRequest {
                program: input.program.into(),
                arguments: input.arguments.into_iter().map(Into::into).collect(),
            };
            let options = ToolOptions {
                keep: input.keep,
                merge_internals: input.merge_internals,
            };
            #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
            let result = tools::qbdi::run_qbdi(request, &options).unwrap();

            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            let result = tools::sde::run_sde(request, &options).unwrap();

            output_result(
                result,
                input.sort,
                input.limit,
                input.breakdown,
                input.write_results_to,
            );
        }
        #[cfg(target_family = "unix")]
        "perf-events" => {
            todo!()
        }
        "install" => {
            #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
            tools::install_qbdi(true, true);

            #[cfg(any(target_os = "linux", target_os = "windows"))]
            tools::install_sde();
        }
        "install-qbdi" => {
            #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
            {
                let mut just_lib = false;
                let mut just_qbdi = false;
                for arg in args {
                    if arg == "--lib" {
                        just_lib = true;
                    } else if arg == "--qbdi" {
                        just_qbdi = true;
                    } else {
                        panic!("unknown {arg:?}");
                    }
                }
                if !just_lib && !just_qbdi {
                    just_lib = true;
                    just_qbdi = true;
                }
                tools::install_qbdi(just_lib, just_qbdi);
            }
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        "install-sde" => {
            tools::install_sde();
        }
        tool => {
            println!("unknown tool {tool:?}. run with 'qbdi', 'sde', 'perf-events' or 'time'");
        }
    }
}

fn output_result(
    result: ToolOutput,
    sort: Option<Sorting>,
    limit: usize,
    breakdown: bool,
    write_results_to: Option<String>,
) {
    match result {
        ToolOutput::SymbolInstructionCounts { symbols, total } => {
            // TODO cloning ...
            print_results(
                &mut std::io::stdout(),
                symbols.clone(),
                total.clone(),
                OutputFormat::default(),
                sort.clone(),
                limit,
                breakdown,
            )
            .unwrap();
            if let Some(path) = write_results_to {
                let path = std::path::Path::new(&path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                let mut file = std::fs::File::create(path).unwrap();
                let format =
                    if let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) {
                        OutputFormat::from_extension(extension).unwrap_or_default()
                    } else {
                        OutputFormat::default()
                    };
                print_results(&mut file, symbols, total, format, sort, limit, breakdown).unwrap();
            }
        }
        _ => todo!(),
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

impl OutputFormat {
    pub fn from_extension(extension: &str) -> Result<Self, &str> {
        match extension {
            "txt" => Ok(Self::Plain),
            "json" => Ok(Self::JSON),
            "csv" => Ok(Self::CSV),
            "md" => Ok(Self::Markdown),
            unknown => Err(unknown),
        }
    }
}

#[derive(Debug)]
pub struct BenchmarkInput {
    /// number of symbol entries to show
    pub limit: usize,
    pub sort: Option<Sorting>,
    // /// plain, JSON, markdown, csv
    // pub format: OutputFormat,
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
            // format: OutputFormat::default(),
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
                // "--format" => {
                //     let format = args.next().expect("no format given");
                //     this.format = match format.as_str() {
                //         "plain" => OutputFormat::Plain,
                //         "json" => OutputFormat::JSON,
                //         "csv" => OutputFormat::CSV,
                //         "markdown" => OutputFormat::Markdown,
                //         format => {
                //             eprintln!("Unknown output format '{format:?}'");
                //             OutputFormat::Plain
                //         }
                //     };
                // }
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
                if breakdown {
                    let mut first = true;
                    for (header, value) in row.statistics.as_rows() {
                        let initial = if std::mem::take(&mut first) { "" } else { ", " };
                        write!(
                            to,
                            "{initial}{header}: {value}",
                            value = count_with_seperator(value as usize)
                        )?;
                    }

                    // TODO
                    // for (name, count) in &row.statistics.others {
                    //     write!(
                    //         to,
                    //         " {name}: {count}",
                    //         count = count_with_seperator(*count as usize)
                    //     )?;
                    // }
                    writeln!(to)?;
                } else {
                    writeln!(
                        to,
                        "total: {count}",
                        count = count_with_seperator(row.statistics.total as usize)
                    )?;
                }
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
                    let mut builder = json_builder_macro::Builder::new(&mut buf);
                    builder.add("symbol_name", row.symbol_name.as_str());
                    for (key, value) in row.statistics.as_rows() {
                        if key == "other" {
                            continue;
                        }
                        builder.add(key, value);
                    }
                    builder.add("other", row.statistics.others.clone());
                    builder.end();
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
                write!(to, "symbol name")?;
                for (header, _) in rows[0].statistics.as_rows() {
                    write!(to, ",{header}")?;
                }
                writeln!(to)?;
            } else {
                writeln!(to, "symbol name,total")?;
            }
            for row in rows {
                let Entry {
                    symbol_name,
                    statistics,
                } = row;
                if breakdown {
                    write!(to, "\"{symbol_name}\"")?;
                    for (_, value) in statistics.as_rows() {
                        write!(to, ",{value}")?;
                    }
                    writeln!(to)?;
                    // TODO ..?
                    // for (name, count) in &row.others {
                    //     write!(
                    //         to,
                    //         ",\"{name}\",{count}",
                    //         count = *count as usize
                    //     )?;
                    // }
                } else {
                    writeln!(to, "\"{symbol_name}\",{total}", total = statistics.total)?;
                }
            }
            Ok(())
        }
        OutputFormat::Markdown => {
            if breakdown {
                let start = rows[0].statistics.as_rows();
                write!(to, "|symbol name")?;
                for (header, _) in &start {
                    write!(to, "|{header}")?;
                }
                writeln!(to, "|")?;
                for _ in 0..=start.len() {
                    write!(to, "|---")?;
                }
                writeln!(to, "|")?;
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
                    write!(to, "\"{symbol_name}\"")?;
                    for (_, value) in statistics.as_rows() {
                        write!(to, ",{value}")?;
                    }
                    writeln!(to)?;
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
