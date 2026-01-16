use std::process::ExitCode;

use log::error;
use onionbell::app::App;

fn main() -> Result<(), ExitCode> {
    #[cfg(debug_assertions)]
    {
        // We want full log in debug builds.
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    }
    #[cfg(not(debug_assertions))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    let app = App::new();
    let Ok(mut app) = app else {
        if let Err(e) = app {
            error!("Application initialization failed: {}", e);
        }
        return Err(ExitCode::FAILURE);
    };
    if let Err(e) = app.run() {
        error!("Fatal error: {}", e);
        return Err(ExitCode::FAILURE);
    }
    Ok(())
}
