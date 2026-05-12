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

    let file = match args.file {
        Some(f) => f,
        None => {
            eprintln!("nbv: missing file argument");
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

    let ctx = env::detect(args.no_color, args.no_images);

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    if let Err(e) = render::render_notebook(&nb, &ctx, &mut w) {
        if e.kind() == io::ErrorKind::BrokenPipe { return ExitCode::SUCCESS; }
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
