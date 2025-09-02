use std::{env::args, fs, io::{self, stdin}, process::exit};

use d6_cmd::{parser, Vm};
use getopts_macro::getopts_options;
use line_column::line_column;

fn main() {
    let options = getopts_options! {
        -h, --help          "show help message";
    };
    let mut args = args();
    let prog_name = args.next().unwrap();
    let matches = match options.parse(args) {
        Ok(matched) => matched,
        Err(e) => {
            eprintln!("{e}");
            exit(3)
        },
    };
    if matches.opt_present("help") {
        let help = options.usage(&format!(
                "Usage: {prog_name} [Options].. [FILE].."
        ));
        print!("{help}");
        return;
    }

    for path in matches.free {
        let readed = if path == "-" {
            io::read_to_string(stdin().lock())
        } else {
            fs::read_to_string(&path)
        };
        let src = match readed {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{prog_name}: {e}");
                exit(e.raw_os_error().unwrap_or(1))
            },
        };
        let cmds = match parser::cmds(&src) {
            Ok(cmds) => cmds,
            Err(e) => {
                eprintln!("{prog_name}: `{path}` parse error {e}");
                exit(4);
            },
        };
        let mut vm = Vm::default();
        match vm.run_to_finish(&mut cmds.iter()) {
            Ok(()) => {},
            Err(e) => {
                let (msg, loc) = match e {
                    d6_cmd::Error::UndefinedMacro(var, loc) => {
                        (format!("undefined macro {var:?}"), loc)
                    },
                    d6_cmd::Error::UndefinedMark(var, loc) => {
                        (format!("undefined mark {var:?}"), loc)
                    },
                };
                let (line, column) = line_column(&src, loc.0);
                eprintln!("{prog_name}: `{path}` runtime error at {line}:{column} {msg}");
                exit(5);
            },
        }
        for (name, value) in &vm.vars {
            println!("{name}: {value:?}");
        }
        println!()
    }
}
