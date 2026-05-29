use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use clap::Parser;

use nbv::cli::{Args, Command};
use nbv::env;
use nbv::generate;
use nbv::ipynb::parse;
use nbv::render;
use nbv::setup;

fn main() -> ExitCode {
    install_sigpipe_handler();
    let args = Args::parse();

    match args.command {
        Some(Command::Setup { yes }) => return ExitCode::from(setup::run(yes) as u8),
        Some(Command::Completion { shell }) => {
            return run_generate(|w| generate::completion(shell, w));
        }
        Some(Command::Mangen) => {
            return run_generate(generate::mangen);
        }
        None => {}
    }

    if args.list_themes {
        let ts = render::code::theme_set();
        let mut names: Vec<&String> = ts.themes.keys().collect();
        names.sort();
        for n in names {
            println!("{}", n);
        }
        return ExitCode::SUCCESS;
    }

    if let Some(name) = &args.theme {
        let ts = render::code::theme_set();
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

    let file = match &args.file {
        Some(f) => f.clone(),
        None => {
            eprintln!("nbv {}", env!("CARGO_PKG_VERSION"));
            eprintln!();
            eprintln!("nbv: no notebook given");
            eprintln!();
            eprintln!("Usage:");
            eprintln!("    nbv [OPTIONS] <FILE>          Render a Jupyter notebook to stdout");
            eprintln!(
                "    nbv setup [--yes]             Add the nbv binary directory to your shell PATH"
            );
            eprintln!("    nbv completion <SHELL>        Generate shell completion script");
            eprintln!("    nbv mangen                    Generate man page to stdout");
            eprintln!();
            eprintln!("Run `nbv --help` for more details.");
            return ExitCode::from(2);
        }
    };

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

    let mut ctx = env::detect(
        // --plain forces --no-color and --no-images
        args.no_color || args.plain,
        args.no_images || args.plain,
        args.theme.clone(),
        args.width.map(|w| w as usize),
    );
    // Belt-and-braces: even if some future env::detect path would re-enable
    // these, --plain semantics must hold.
    if args.plain {
        ctx.use_color = false;
        ctx.image_backend = env::ImageBackend::Placeholder;
    }

    let filters = build_filters(&args, nb.cells.len());

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    if let Err(e) = render::render_notebook(&nb, &filters, &ctx, &mut w) {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return ExitCode::SUCCESS;
        }
        eprintln!("nbv: write error: {}", e);
        return ExitCode::from(1);
    }
    let _ = w.flush();
    ExitCode::SUCCESS
}

fn build_filters(args: &Args, total_cells: usize) -> render::RenderFilters {
    let cells_range = args.cells.map(|(start, end)| {
        let s = (start.get() - 1).min(total_cells);
        let e = end.get().min(total_cells);
        s..e
    });
    render::RenderFilters {
        cells_range,
        no_output: args.no_output || args.code_only,
        code_only: args.code_only,
        plain: args.plain,
    }
}

fn run_generate<F>(f: F) -> ExitCode
where
    F: FnOnce(&mut dyn Write) -> io::Result<()>,
{
    let stdout = io::stdout();
    let mut w = stdout.lock();
    match f(&mut w) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nbv: write error: {}", e);
            ExitCode::from(1)
        }
    }
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
