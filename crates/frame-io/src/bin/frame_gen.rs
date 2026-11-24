//! `frame-gen`: writes a synthetic raw frame stream to standard output. See `frame_io::cli`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let code = frame_io::cli::run_gen(&args, &mut out, &mut std::io::stderr());
    std::process::exit(code);
}
