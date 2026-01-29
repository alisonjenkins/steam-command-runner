pub mod config;
pub mod gamescope;
pub mod install;
pub mod launch_options;
pub mod proton;
pub mod run;
pub mod search;

pub use config::handle_config;
pub use gamescope::handle_gamescope;
pub use install::{handle_install, handle_uninstall};
pub use launch_options::handle_launch_options;
pub use proton::handle_proton;
pub use run::handle_run;
pub use search::handle_search;
