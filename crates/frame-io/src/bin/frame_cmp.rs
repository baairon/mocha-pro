//! `frame-cmp`: compares two raw frame streams exactly. See `frame_io::cli`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdin = std::io::stdin();
    let code = frame_io::cli::run_cmp(
        &args,
        &mut stdin.lock(),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    std::process::exit(code);
}
