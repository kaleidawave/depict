use std::io::BufReader;
use std::process::{Command, Stdio};

pub const TEMP_FILE: &str = "sde-out.txt";

pub fn run_sde(
    request: crate::CommandRequest,
    options: &crate::ToolOptions,
) -> Result<crate::ToolOutput, ()> {
    let file_path: &str = options.keep.as_deref().unwrap_or(TEMP_FILE);

    // TODO hmm
    let blocks = 50;

    {
        let sde_path = if let Some(path) = crate::adjacent_sde_path()
            && let Ok(true) = std::fs::exists(&path)
        {
            path
        } else if let Ok(dir) = std::env::var("SDE_PATH") {
            format!("{dir}/sde")
        } else {
            String::from("sde")
        };

        {
            let mut command = Command::new(&sde_path);
            command.args(["-help"]);
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = command.spawn().expect("could not spawn SDE (or it failed)");
            let _ = child.wait().unwrap();
        }

        let mut command = Command::new(sde_path);
        command.args([
            "-omix",
            file_path,
            "-mix_filter_no_shared_libs",
            "-top_blocks",
            &blocks.to_string(),
            "--",
        ]);
        command.arg(request.program);
        command.args(request.arguments);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("could not spawn SDE (or it failed)");
        let _ = child.wait().unwrap();
    }

    let file = std::fs::File::open(file_path).expect("sde did not create file");

    let out = BufReader::new(file);

    // TODO want options.merge_internals, not skip internals
    let rows = sde_output_parser::parse(out, false);

    let symbols: Vec<_> = rows
        .into_iter()
        .map(|(symbol_name, item)| crate::Entry {
            symbol_name,
            statistics: crate::Statistics {
                total: item.total,
                mem_read: item.mem_read,
                mem_write: item.mem_write,
                stack_read: item.stack_read,
                stack_write: item.stack_write,
                call: item.call,
                others: Default::default(),
            },
        })
        .collect();

    let total: crate::Statistics =
        symbols
            .iter()
            .fold(crate::Statistics::default(), |mut acc, row| {
                acc += row.statistics.clone();
                acc
            });

    if options.keep.is_none() {
        std::fs::remove_file(file_path).unwrap();
    }

    Ok(crate::ToolOutput::SymbolInstructionCounts { total, symbols })
}
