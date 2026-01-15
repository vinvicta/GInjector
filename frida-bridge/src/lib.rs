//! Frida Bridge Library
//!
//! Native GS2 bytecode injection using Frida.

/// Graal client type for injection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    GraalV6,
    GraalWorlds,
}

impl ClientType {
    pub fn name(&self) -> &'static str {
        match self {
            ClientType::GraalV6 => "Graal V6",
            ClientType::GraalWorlds => "Graal Worlds",
        }
    }

    pub fn target_module(&self) -> &'static str {
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

    /// Offset to TGraalVar constructor
    pub fn tgralvar_constructor_offset(&self) -> usize {
        match self {
            ClientType::GraalV6 => 0x195770,
            ClientType::GraalWorlds => 0x9A340,
        }
    }

    /// Offset to TGraalVar::SetScript method
    pub fn tgralvar_setscript_offset(&self) -> usize {
        match self {
            ClientType::GraalV6 => 0x196290,
            ClientType::GraalWorlds => 0x9EDE0,
        }
    }

    /// Whether to use thiscall calling convention
    pub fn uses_thiscall(&self) -> bool {
        match self {
            ClientType::GraalV6 => true,
            ClientType::GraalWorlds => false,
        }
    }

    /// Magic number check offset (V6 only)
    pub fn magic_check_offset(&self) -> Option<usize> {
        match self {
            ClientType::GraalV6 => Some(0x17da90),
            ClientType::GraalWorlds => None,
        }
    }

    /// Magic number value to wait for
    pub fn magic_check_value(&self) -> Option<u32> {
        match self {
            ClientType::GraalV6 => Some(157876074),
            ClientType::GraalWorlds => None,
        }
    }
}

/// Convert bytecode to space-separated hex string
pub fn bytecode_to_hex(bytecode: &[u8]) -> String {
    bytecode
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convert space-separated hex string to bytecode
pub fn hex_to_bytecode(hex: &str) -> Result<Vec<u8>, String> {
    hex.split_whitespace()
        .map(|s| {
            u8::from_str_radix(s, 16).map_err(|e| format!("Invalid hex: {} - {}", s, e))
        })
        .collect()
}

/// GS2 String structure as used in memory
///
/// Layout (what create_tstring produces):
/// - Reference pointer (8 bytes): points to data below
/// - Data structure:
///   - [0-3]: length (u32)
///   - [4-7]: value/flags (always 100 for GS2 strings)
///   - [8-]: string bytes
///   - [8+len]: null terminator
#[repr(C)]
pub struct GS2String {
    pub length: u32,
    pub value: u32,
    // Followed by string data
}

/// Custom injection offsets
#[derive(Debug, Clone)]
pub struct InjectionOffsets {
    pub constructor_offset: usize,
    pub setscript_offset: usize,
    pub uses_thiscall: bool,
    pub magic_check_offset: Option<usize>,
    pub magic_check_value: Option<u32>,
}

/// Native Frida injector for GS2 bytecode
pub struct FridaInjector {
    client_type: ClientType,
    custom_offsets: Option<InjectionOffsets>,
}

impl FridaInjector {
    /// Create a new injector for the specified client type
    pub fn new(client_type: ClientType) -> Self {
        Self { client_type, custom_offsets: None }
    }

    /// Create a new injector with custom offsets
    pub fn with_offsets(client_type: ClientType, offsets: InjectionOffsets) -> Self {
        Self { client_type, custom_offsets: Some(offsets) }
    }

    /// Set custom offsets
    pub fn set_offsets(&mut self, offsets: InjectionOffsets) {
        self.custom_offsets = Some(offsets);
    }

    /// Get the constructor offset to use (custom or default)
    fn get_constructor_offset(&self) -> usize {
        self.custom_offsets
            .as_ref()
            .map(|o| o.constructor_offset)
            .unwrap_or_else(|| self.client_type.tgralvar_constructor_offset())
    }

    /// Get the setscript offset to use (custom or default)
    fn get_setscript_offset(&self) -> usize {
        self.custom_offsets
            .as_ref()
            .map(|o| o.setscript_offset)
            .unwrap_or_else(|| self.client_type.tgralvar_setscript_offset())
    }

    /// Get whether to use thiscall (custom or default)
    fn get_uses_thiscall(&self) -> bool {
        self.custom_offsets
            .as_ref()
            .map(|o| o.uses_thiscall)
            .unwrap_or_else(|| self.client_type.uses_thiscall())
    }

    /// Get the magic check offset (custom or default)
    fn get_magic_check_offset(&self) -> Option<usize> {
        self.custom_offsets
            .as_ref()
            .and_then(|o| o.magic_check_offset)
            .or_else(|| self.client_type.magic_check_offset())
    }

    /// Get the magic check value (custom or default)
    fn get_magic_check_value(&self) -> Option<u32> {
        self.custom_offsets
            .as_ref()
            .and_then(|o| o.magic_check_value)
            .or_else(|| self.client_type.magic_check_value())
    }

    /// Check if Frida is installed and available
    pub async fn check_frida_installed(&self) -> Result<bool, std::io::Error> {
        let output = tokio::process::Command::new("frida")
            .arg("--version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// Check if the target process is running
    pub async fn check_process_running(&self) -> Result<bool, std::io::Error> {
        let output = tokio::process::Command::new("frida-ps")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(false);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let target = self.client_type.target_module();

        // Check if the process/module is in the list
        for line in output_str.lines() {
            if line.contains(target) || line.contains(target.trim_end_matches(".dll")) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Generate a Frida script that injects the bytecode
    ///
    /// This generates the JavaScript code that performs the actual injection,
    /// modeled after the logic in graalv6_gs2.js and worldsclient_gs2.js
    pub fn generate_injection_script(
        &self,
        bytecode_hex: &str,
        variable_name: &str,
    ) -> String {
        let module_name = self.client_type.target_module();
        let constructor_offset = format!("0x{:X}", self.get_constructor_offset());
        let setscript_offset = format!("0x{:X}", self.get_setscript_offset());
        let uses_thiscall = self.get_uses_thiscall();
        let var_name_escaped = variable_name.replace('\\', "\\\\").replace('\'', "\\'");

        let magic_check_offset = self.get_magic_check_offset();
        let magic_check_value = self.get_magic_check_value();

        // Build script using the working pattern from graalv6_gs2.js
        let mut script = format!("// Generated by GraalHax - GS2 Bytecode Injector for {}\n\n", self.client_type.name());

        script.push_str("const alloc_mem = (size) => Memory.alloc(size);\n\n");

        script.push_str("function create_tstring(e) {\n");
        script.push_str("    const r = Memory.alloc(8);\n");
        script.push_str("    const a = Memory.alloc(e.length + 8 + 10);\n");
        script.push_str("    r.writePointer(a);\n");
        script.push_str("    a.writeInt(e.length);\n");
        script.push_str("    ptr(parseInt(a) + 4).writeInt(100);\n");
        script.push_str("\n");
        script.push_str("    for (let i = 0; i < e.length; i++) {\n");
        script.push_str("        ptr(parseInt(a) + 8).add(i).writeU8(e.charCodeAt(i));\n");
        script.push_str("    }\n");
        script.push_str("    ptr(parseInt(a) + 8).add(e.length).writeU8(0);\n");
        script.push_str("    return r;\n");
        script.push_str("}\n\n");

        script.push_str("const hexToBytecode = (hex) => {\n");
        script.push_str("    return hex.split(\" \").map(byte => String.fromCharCode(parseInt(byte, 16))).join(\"\");\n");
        script.push_str("};\n\n");

        script.push_str("async function init() {\n");
        script.push_str("    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));\n");
        script.push_str(&format!("    var baseAddress = Process.findModuleByName(\"{}\").base;\n", module_name));
        script.push_str("    var check_address = baseAddress.add(0x");

        // Add magic check or just skip to address definition
        if let (Some(offset), Some(value)) = (magic_check_offset, magic_check_value) {
            script.push_str(&format!("{:X});\n", offset));
            script.push_str(&format!("    while (check_address.readInt() !== {}) {{\n", value));
            script.push_str("        await sleep(100);\n");
            script.push_str("    }\n\n");
        } else {
            script.push_str("0);\n\n");
        }

        script.push_str("    const TGraalVar_TGraalVar = new NativeFunction(baseAddress.add(");
        script.push_str(&constructor_offset);
        if uses_thiscall {
            script.push_str("), 'void', ['pointer', 'pointer'], 'thiscall');\n");
        } else {
            script.push_str("), 'void', ['pointer', 'pointer']);\n");
        }
        script.push_str("    const TGraalVar_SetScript = new NativeFunction(baseAddress.add(");
        script.push_str(&setscript_offset);
        if uses_thiscall {
            script.push_str("), 'void', ['pointer', 'pointer'], 'thiscall');\n\n");
        } else {
            script.push_str("), 'void', ['pointer', 'pointer']);\n\n");
        }
        script.push_str("    console.log(\"Waiting 10 seconds before injecting script...\");\n");
        script.push_str("    await sleep(10000);\n");
        script.push_str("    console.log(\"Injecting script...\");\n\n");
        script.push_str("    const variable = alloc_mem(0x1000);\n");
        script.push_str("    console.log(\"Allocated memory for variable\");\n");
        script.push_str(&format!("    TGraalVar_TGraalVar(variable, create_tstring(\"{}\"));\n", var_name_escaped));
        script.push_str("    console.log(\"Created TGraalVar instance\");\n\n");
        script.push_str(r#"    TGraalVar_SetScript(variable, create_tstring(hexToBytecode(""#);
        script.push_str(bytecode_hex);
        script.push_str(r#"")));"#);
        script.push_str("\n\n");
        script.push_str("    console.log(\"SCRIPT INJECTED\");\n");
        script.push_str("}\n\n");
        script.push_str("init();\n");

        script
    }

    /// Inject bytecode into the target process
    ///
    /// This generates the Frida script and executes it against the running process
    pub async fn inject(
        &self,
        bytecode: &[u8],
        variable_name: &str,
    ) -> Result<String, String> {
        // Check if Frida is installed
        if !self.check_frida_installed().await.map_err(|e| e.to_string())? {
            return Err("Frida is not installed".to_string());
        }

        // Check if process is running
        if !self.check_process_running().await.map_err(|e| e.to_string())? {
            return Err(format!(
                "Target process {} is not running",
                self.client_type.target_module()
            ));
        }

        let hex = bytecode_to_hex(bytecode);
        let script = self.generate_injection_script(&hex, variable_name);

        // Write script to temp file
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("graalhax_inject.js");
        std::fs::write(&script_path, script)
            .map_err(|e| format!("Failed to write script: {}", e))?;

        // Execute Frida with the generated script
        let output = tokio::process::Command::new("frida")
            .arg("-l")
            .arg(&script_path)
            .arg(self.client_type.target_module())
            .arg("--exit-on-error")
            .output()
            .await;

        // Clean up temp script
        let _ = std::fs::remove_file(&script_path);

        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Ok(format!("Success: {}\n{}", stdout, stderr))
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(format!("Injection failed: {}", stderr))
                }
            }
            Err(e) => Err(format!("Failed to execute Frida: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytecode_to_hex() {
        let bytecode = vec![0x00, 0x01, 0x02, 0xFF, 0xAB];
        let hex = bytecode_to_hex(&bytecode);
        assert_eq!(hex, "00 01 02 FF AB");
    }

    #[test]
    fn test_hex_to_bytecode() {
        let hex = "00 01 02 FF AB";
        let bytecode = hex_to_bytecode(hex).unwrap();
        assert_eq!(bytecode, vec![0x00, 0x01, 0x02, 0xFF, 0xAB]);
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = vec![0x00, 0x42, 0xFF, 0x10, 0x20];
        let hex = bytecode_to_hex(&original);
        let decoded = hex_to_bytecode(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_client_type_v6_offsets() {
        let client = ClientType::GraalV6;
        assert_eq!(client.target_module(), "Graal.exe");
        assert_eq!(client.tgralvar_constructor_offset(), 0x195770);
        assert_eq!(client.tgralvar_setscript_offset(), 0x196290);
        assert_eq!(client.magic_check_offset(), Some(0x17da90));
        assert_eq!(client.magic_check_value(), Some(157876074));
        assert!(client.uses_thiscall());
    }

    #[test]
    fn test_client_type_worlds_offsets() {
        let client = ClientType::GraalWorlds;
        assert_eq!(client.target_module(), "Worlds.exe");
        assert_eq!(client.tgralvar_constructor_offset(), 0x9A340);
        assert_eq!(client.tgralvar_setscript_offset(), 0x9EDE0);
        assert_eq!(client.magic_check_offset(), None);
        assert_eq!(client.magic_check_value(), None);
        assert!(!client.uses_thiscall());
    }

    #[test]
    fn test_generate_injection_script_v6() {
        let injector = FridaInjector::new(ClientType::GraalV6);
        let script = injector.generate_injection_script("00 01 02", "TestVar");

        assert!(script.contains("Graal V6"));
        assert!(script.contains("0x195770"));
        assert!(script.contains("0x196290"));
        assert!(script.contains("thiscall"));
        assert!(script.contains("157876074"));
        assert!(script.contains("TestVar"));
        assert!(script.contains("00 01 02"));
        // Key fix: 10 second wait before injection
        assert!(script.contains("await sleep(10000)"));
        assert!(script.contains("Waiting 10 seconds before injecting"));
    }

    #[test]
    fn test_generate_injection_script_worlds() {
        let injector = FridaInjector::new(ClientType::GraalWorlds);
        let script = injector.generate_injection_script("00 01 02", ".");

        assert!(script.contains("Graal Worlds"));
        assert!(script.contains("0x9A340"));
        assert!(script.contains("0x9EDE0"));
        assert!(!script.contains("thiscall"));
        assert!(!script.contains("157876074"));
    }
}
