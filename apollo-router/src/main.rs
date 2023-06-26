//! Main entry point for CLI command to start server.
//! // add custom plguns here
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
