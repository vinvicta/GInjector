//! Configuration management for GInjector

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Graal client type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientType {
    GraalWorlds,
    EraSteam,
}

impl Default for ClientType {
    fn default() -> Self {
        Self::GraalWorlds
    }
}

impl ClientType {
    pub fn name(&self) -> &'static str {
        match self {
            ClientType::GraalWorlds => "Graal Worlds",
            ClientType::EraSteam => "Era (Steam)",
        }
    }

    pub fn target_process(&self) -> &'static str {
        match self {
            ClientType::GraalWorlds => "Worlds.exe",
            ClientType::EraSteam => "Era.exe",
        }
    }

    pub fn default_variable_name(&self) -> &'static str {
        match self {
            ClientType::GraalWorlds => ".",
            ClientType::EraSteam => ".",
        }
    }

    /// Get default offsets for this client type
    pub fn default_offsets(&self) -> ClientOffsets {
        match self {
            ClientType::GraalWorlds => ClientOffsets {
                constructor_offset: "0x9A340".to_string(),
                setscript_offset: "0x9EDE0".to_string(),
                uses_thiscall: false,
                magic_check_offset: None,
                magic_check_value: None,
                use_pattern_scanning: false,
                constructor_pattern: None,
                setscript_pattern: None,
                pattern_index: None,
            },
            ClientType::EraSteam => ClientOffsets {
                // Era (Steam) uses pattern scanning from the ESP hack
                constructor_offset: "0x0".to_string(),
                setscript_offset: "0x0".to_string(),
                uses_thiscall: false,
                magic_check_offset: None,
                magic_check_value: None,
                use_pattern_scanning: true,
                constructor_pattern: Some(
                    "40 53 48 83 EC 20 48 8B D9 E8 ?? ?? ?? ?? 48 ?? ?? ?? ?? ?? ?? C7 ?? ?? ?? 00 00 00 48 ?? ?? ?? ?? ?? ?? ?? ?? 66 C7".to_string()
                ),
                setscript_pattern: Some(
                    "48 89 ?? ?? ?? 57 48 ?? ?? ?? 48 8B DA 48 8B F9 E8 ?? ?? ?? ?? ?? ?? ?? ?? 48 ?? ?? 48 ?? ?? ?? ?? 48 ?? ?? ?? 5F".to_string()
                ),
                pattern_index: Some(0),
            },
        }
    }

    /// Get all client types as a slice for dropdowns
    pub fn all() -> &'static [ClientType] {
        &[ClientType::GraalWorlds, ClientType::EraSteam]
    }
}

/// Memory offsets for a specific client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientOffsets {
    /// TGraalVar constructor offset (hex string, e.g., "0x195770")
    pub constructor_offset: String,

    /// TGraalVar::SetScript offset (hex string, e.g., "0x196290")
    pub setscript_offset: String,

    /// Whether to use thiscall calling convention
    pub uses_thiscall: bool,

    /// Magic check offset for V6 (optional)
    pub magic_check_offset: Option<String>,

    /// Magic check value for V6 (optional)
    pub magic_check_value: Option<u32>,

    /// Whether to use pattern scanning instead of static offsets
    /// (Era/Steam uses Memory.scanSync to find addresses)
    pub use_pattern_scanning: bool,

    /// Byte pattern for TGraalVar constructor (e.g., "40 53 48 83 EC 20 48 8B D9 ?? ?? ?? ??")
    pub constructor_pattern: Option<String>,

    /// Byte pattern for TGraalVar::SetScript (e.g., "48 89 ?? ?? ?? 57 48 ?? ?? ?? 48 8B DA")
    pub setscript_pattern: Option<String>,

    /// Pattern match index (which occurrence to use if multiple matches)
    pub pattern_index: Option<usize>,
}

impl ClientOffsets {
    /// Parse hex string to usize
    pub fn parse_offset(hex: &str) -> Result<usize, String> {
        let hex = hex.trim_start_matches("0x").trim_start_matches("0X");
        usize::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex: {}", e))
    }

    /// Get constructor offset as usize
    pub fn constructor_offset_usize(&self) -> Result<usize, String> {
        Self::parse_offset(&self.constructor_offset)
    }

    /// Get setscript offset as usize
    pub fn setscript_offset_usize(&self) -> Result<usize, String> {
        Self::parse_offset(&self.setscript_offset)
    }

    /// Get magic check offset as usize (if available)
    pub fn magic_check_offset_usize(&self) -> Result<Option<usize>, String> {
        match &self.magic_check_offset {
            Some(offset) => Ok(Some(Self::parse_offset(offset)?)),
            None => Ok(None),
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

    /// Custom offsets for each client type
    pub offsets: OffsetsConfig,

    /// Editor settings
    pub editor: EditorConfig,

    /// Color theme
    pub theme: Theme,
}

/// Custom offsets configuration for each client type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetsConfig {
    /// Custom offsets for Graal Worlds (null means use defaults)
    pub graalworlds: Option<ClientOffsets>,

    /// Custom offsets for Era Steam (null means use defaults)
    pub era_steam: Option<ClientOffsets>,
}

impl Default for OffsetsConfig {
    fn default() -> Self {
        Self {
            graalworlds: None,
            era_steam: None,
        }
    }
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
            client_type: ClientType::GraalWorlds,
            default_variable_name: None,
            offsets: OffsetsConfig::default(),
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

    /// Get the offsets to use for the current client type
    /// Returns custom offsets if set, otherwise returns defaults
    pub fn get_offsets(&self) -> ClientOffsets {
        match self.client_type {
            ClientType::GraalWorlds => {
                self.offsets.graalworlds
                    .clone()
                    .unwrap_or_else(|| self.client_type.default_offsets())
            }
            ClientType::EraSteam => {
                self.offsets.era_steam
                    .clone()
                    .unwrap_or_else(|| self.client_type.default_offsets())
            }
        }
    }

    /// Set custom offsets for the current client type
    pub fn set_offsets(&mut self, offsets: ClientOffsets) {
        match self.client_type {
            ClientType::GraalWorlds => {
                self.offsets.graalworlds = Some(offsets);
            }
            ClientType::EraSteam => {
                self.offsets.era_steam = Some(offsets);
            }
        }
    }

    /// Reset offsets for the current client type to defaults
    pub fn reset_offsets(&mut self) {
        match self.client_type {
            ClientType::GraalWorlds => {
                self.offsets.graalworlds = None;
            }
            ClientType::EraSteam => {
                self.offsets.era_steam = None;
            }
        }
    }

    /// Check if current client is using custom offsets
    pub fn has_custom_offsets(&self) -> bool {
        match self.client_type {
            ClientType::GraalWorlds => self.offsets.graalworlds.is_some(),
            ClientType::EraSteam => self.offsets.era_steam.is_some(),
        }
    }

    /// Check if a specific client type is using custom offsets
    pub fn has_custom_offsets_for(&self, client_type: ClientType) -> bool {
        match client_type {
            ClientType::GraalWorlds => self.offsets.graalworlds.is_some(),
            ClientType::EraSteam => self.offsets.era_steam.is_some(),
        }
    }
}

