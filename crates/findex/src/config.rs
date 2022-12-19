use crate::gui::dialog::show_dialog;
use abi_stable::std_types::{RHashMap, RString};
use findex_plugin::findex_internal::{load_plugin, PluginDefinition};
use gtk::MessageType;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(debug_assertions))]
use shellexpand::tilde;

lazy_static! {
    pub static ref FINDEX_CONFIG: FindexConfig = {
        let settings = load_settings();
        if let Err(e) = settings {
            FindexConfig {
                error: RString::from(e),
                ..Default::default()
            }
        } else {
            settings.unwrap()
        }
    };
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct FindexConfig {
    pub default_window_width: i32,
    pub min_content_height: i32,
    pub max_content_height: i32,
    pub name_match_highlight_color: RString,
    pub decorate_window: bool,
    pub close_window_on_losing_focus: bool,
    pub query_placeholder: RString,
    pub icon_size: i32,
    pub toggle_key: RString,
    pub min_score: isize,
    pub result_size: usize,
    pub plugins: HashMap<RString, Plugin>,
    #[serde(skip)]
    pub error: RString,
    /// This should get filled after configuration gets initialized
    #[serde(skip)]
    pub plugin_definitions: HashMap<RString, PluginDefinition>,
}

#[derive(Serialize, Deserialize)]
pub struct Plugin {
    pub prefix: Option<RString>,
    pub path: RString,
    pub config: RHashMap<RString, RString>,
}

impl Default for FindexConfig {
    fn default() -> Self {
        #[cfg(not(debug_assertions))]
        let plugins_path = tilde("~/.config/findex/plugins").to_string();

        FindexConfig {
            min_content_height: 0,
            max_content_height: 400,
            default_window_width: 600,
            name_match_highlight_color: RString::from("orange"),
            decorate_window: false,
            query_placeholder: RString::from("Search for applications"),
            close_window_on_losing_focus: true,
            icon_size: 32,
            min_score: 5,
            result_size: 10,
            toggle_key: RString::from("<Shift>space"),
            error: RString::new(),
            #[cfg(not(debug_assertions))]
            plugins: HashMap::from([
                (
                    RString::from("github-repo"),
                    Plugin {
                        prefix: None,
                        path: RString::from(format!("{plugins_path}/github_repo.so")),
                        config: RHashMap::new(),
                    },
                ),
                (
                    RString::from("urlopen"),
                    Plugin {
                        prefix: None,
                        path: RString::from(format!("{plugins_path}/urlopen.so")),
                        config: RHashMap::new(),
                    },
                ),
            ]),
            #[cfg(debug_assertions)]
            plugins: HashMap::from([
                (
                    RString::from("github-repo"),
                    Plugin {
                        prefix: None,
                        path: RString::from("plugins/github-repo/target/debug/libgithub_repo.so"),
                        config: RHashMap::new(),
                    },
                ),
                (
                    RString::from("urlopen"),
                    Plugin {
                        prefix: None,
                        path: RString::from("plugins/urlopen/target/debug/liburlopen.so"),
                        config: RHashMap::new(),
                    },
                ),
            ]),
            plugin_definitions: HashMap::new(),
        }
    }
}

fn load_settings() -> Result<FindexConfig, String> {
    #[cfg(debug_assertions)]
    let settings_path = String::from("settings.toml");

    #[cfg(not(debug_assertions))]
    let settings_path = shellexpand::tilde("~/.config/findex/settings.toml").to_string();

    #[cfg(not(debug_assertions))]
    let settings_dir = shellexpand::tilde("~/.config/findex").to_string();

    let file = std::path::Path::new(&settings_path);
    let mut res = if !file.exists() {
        #[cfg(not(debug_assertions))]
        if !std::path::Path::new(&settings_dir).exists() {
            std::fs::create_dir(&settings_dir).unwrap();
        }

        let settings = toml::to_string(&FindexConfig::default()).unwrap();
        std::fs::write(settings_path, settings).unwrap();

        Ok(FindexConfig::default())
    } else {
        let settings = std::fs::read_to_string(&settings_path).unwrap();

        let config: FindexConfig =
            toml::from_str(&settings).map_err(|e| format!("Error while parsing settings: {e}"))?;

        std::fs::write(&settings_path, toml::to_string(&config).unwrap()).unwrap();

        Ok(config)
    };

    if let Ok(ref mut config) = res {
        for (name, plugin) in &config.plugins {
            let plugin_definition = match unsafe { load_plugin(&plugin.path) } {
                Ok(pd) => pd,
                Err(e) => {
                    show_dialog(
                        "Error",
                        &format!("Failed to load plugin {name}: {e}"),
                        MessageType::Error,
                    );
                    continue;
                }
            };

            if !unsafe { plugin_definition.plugin_init(&plugin.config) } {
                show_dialog(
                    "Error",
                    &format!("Plugin \"{name}\" failed to initialize"),
                    MessageType::Error,
                );
            }

            config.plugin_definitions.insert(
                plugin
                    .prefix
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| plugin_definition.prefix.clone()),
                plugin_definition,
            );
        }
    }

    res
}
