pub mod compat;
pub mod config;
pub mod install;
pub mod run;
pub mod search;
pub mod uninstall;

pub use compat::handle_compat;
pub use config::handle_config;
pub use install::handle_install;
pub use run::handle_run;
pub use search::handle_search;
pub use uninstall::handle_uninstall;
