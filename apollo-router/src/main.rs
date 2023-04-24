//! Main entry point for CLI command to start server.
mod graph_allow_list;

fn main() {
    match apollo_router::main() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
