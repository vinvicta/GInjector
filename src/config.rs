//! Configuration management for GraalHax TUI

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Graal client type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientType {
    GraalV6,
    GraalWorlds,
}

impl Default for ClientType {
    fn default() -> Self {
        Self::GraalV6
    }
}

impl ClientType {
    pub fn name(&self) -> &'static str {
        match self {
            ClientType::GraalV6 => "Graal V6",
            ClientType::GraalWorlds => "Graal Worlds",
        }
    }

    pub fn target_process(&self) -> &'static str {
        match self {
            ClientType::GraalV6 => "Graal.exe",
            ClientType::GraalWorlds => "Worlds.exe",
        }
    }

    pub fn default_variable_name(&self) -> &'static str {
        match self {
            ClientType::GraalV6 => "VarName",
            ClientType::GraalWorlds => ".",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the GS2 compiler binary
    pub gs2_compiler_path: PathBuf,

    /// Current client type
    pub client_type: ClientType,

    /// Default variable name for injection (overrides client default)
    pub default_variable_name: Option<String>,

    /// Editor settings
    pub editor: EditorConfig,

    /// Color theme
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Enable line numbers
    pub line_numbers: bool,

    /// Tab width
    pub tab_width: usize,

    /// Use spaces for tabs
    pub use_spaces: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub background: String,
    pub foreground: String,
    pub primary: String,
    pub secondary: String,
    pub error: String,
    pub warning: String,
    pub success: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gs2_compiler_path: PathBuf::from("./gs2-parser/bin/gs2test"),
            client_type: ClientType::GraalV6,
            default_variable_name: None,
            editor: EditorConfig {
                line_numbers: true,
                tab_width: 4,
                use_spaces: true,
            },
            theme: Theme {
                background: "#1a1b26".to_string(),
                foreground: "#a9b1d6".to_string(),
                primary: "#7aa2f7".to_string(),
                secondary: "#bb9af7".to_string(),
                error: "#f7768e".to_string(),
                warning: "#e0af68".to_string(),
                success: "#9ece6a".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, anyhow::Error> {
        if let Ok(config_str) = std::fs::read_to_string("config.toml") {
            toml::from_str(&config_str).map_err(Into::into)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let config_str = toml::to_string_pretty(self)?;
        std::fs::write("config.toml", config_str)?;
        Ok(())
    }

    pub fn target_process(&self) -> String {
        self.client_type.target_process().to_string()
    }

    pub fn variable_name(&self) -> String {
        if let Some(ref name) = self.default_variable_name {
            name.clone()
        } else {
            self.client_type.default_variable_name().to_string()
        }
    }
}

