use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use clap::Parser;

use nbv::cli::{Args, Command};
use nbv::env;
use nbv::ipynb::parse;
use nbv::render;
use nbv::setup;

fn main() -> ExitCode {
    install_sigpipe_handler();
    let args = Args::parse();

    if let Some(Command::Setup { yes }) = args.command {
        return ExitCode::from(setup::run(yes) as u8);
    }

    if args.list_themes {
        let ts = syntect::highlighting::ThemeSet::load_defaults();
        let mut names: Vec<&String> = ts.themes.keys().collect();
        names.sort();
        for n in names {
            println!("{}", n);
        }
        return ExitCode::SUCCESS;
    }

    if let Some(name) = &args.theme {
        let ts = syntect::highlighting::ThemeSet::load_defaults();
        if !ts.themes.contains_key(name) {
            eprintln!("nbv: unknown theme '{}'. Available themes:", name);
            let mut names: Vec<&String> = ts.themes.keys().collect();
            names.sort();
            for n in names {
                eprintln!("  {}", n);
            }
            return ExitCode::from(2);
        }
    }

    let file = match args.file {
        Some(f) => f,
        None => {
            eprintln!("nbv: no file given");
            eprintln!();
            eprintln!("Usage:");
            eprintln!("    nbv [OPTIONS] <FILE.ipynb>      Render a Jupyter notebook to stdout");
            eprintln!("    nbv [OPTIONS] <FILE.md>         Render a Markdown document to stdout");
            eprintln!(
                "    nbv setup [--yes]               Add the nbv binary directory to your shell PATH"
            );
            eprintln!();
            eprintln!("Run `nbv --help` for more details.");
            return ExitCode::from(2);
        }
    };

    let ext = file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    let ctx = env::detect(
        args.no_color,
        args.no_images,
        args.theme.clone(),
        args.width.map(|w| w as usize),
    );

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());

    let write_result: io::Result<()> = match ext.as_deref() {
        Some("ipynb") => {
            let nb = match parse::from_path(&file) {
                Err(e) => {
                    eprintln!("nbv: {}: {}", file.display(), e);
                    return ExitCode::from(1);
                }
                Ok(Err(e)) => {
                    eprintln!("nbv: failed to parse '{}': {}", file.display(), e);
                    return ExitCode::from(3);
                }
                Ok(Ok(nb)) => nb,
            };
            render::render_notebook(&nb, &ctx, &mut w)
        }
        Some("md") | Some("markdown") => {
            let source = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("nbv: {}: {}", file.display(), e);
                    return ExitCode::from(1);
                }
            };
            render::render_markdown_doc(&source, &ctx, &mut w)
        }
        _ => {
            eprintln!(
                "nbv: {}: unsupported file type. Supported: .ipynb, .md, .markdown",
                file.display()
            );
            return ExitCode::from(2);
        }
    };

    if let Err(e) = write_result {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return ExitCode::SUCCESS;
        }
        eprintln!("nbv: write error: {}", e);
        return ExitCode::from(1);
    }
    let _ = w.flush();
    ExitCode::SUCCESS
}

fn install_sigpipe_handler() {
    // SIGPIPE를 받으면 즉시 0으로 종료 (e.g. `nbv x.ipynb | head`)
    use signal_hook::consts::SIGPIPE;
    use signal_hook::iterator::Signals;
    let mut signals = Signals::new([SIGPIPE]).expect("install SIGPIPE handler");
    std::thread::spawn(move || {
        let _ = signals.forever().next();
        std::process::exit(0);
    });
}
