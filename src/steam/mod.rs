pub mod installed_games;
pub mod localconfig;
pub mod userdata;

pub use installed_games::{find_installed_games, InstalledGame};
pub use localconfig::{
    generate_default_launch_options, get_launch_options, is_our_launch_options, read_localconfig,
    set_launch_options, write_localconfig, LocalConfig,
};
pub use userdata::{find_user_ids, get_localconfig_path};
