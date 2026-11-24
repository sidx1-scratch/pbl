/// PBL Compiler entry point.

fn main() {
    let exit_code = pbl::cli::run();
    std::process::exit(exit_code);
}
