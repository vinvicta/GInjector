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

/// Native Frida injector for GS2 bytecode
pub struct FridaInjector {
    client_type: ClientType,
}

impl FridaInjector {
    /// Create a new injector for the specified client type
    pub fn new(client_type: ClientType) -> Self {
        Self { client_type }
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
        let constructor_offset = format!("0x{:X}", self.client_type.tgralvar_constructor_offset());
        let setscript_offset = format!("0x{:X}", self.client_type.tgralvar_setscript_offset());
        let var_name_escaped = variable_name.replace('\\', "\\\\").replace('\'', "\\'");
        let thiscall_suffix = if self.client_type.uses_thiscall() { ", 'thiscall'" } else { "" };

        let magic_check = if let (Some(offset), Some(value)) = (
            self.client_type.magic_check_offset(),
            self.client_type.magic_check_value()
        ) {
            format!(
                "// Wait for client initialization\n    var checkAddress = baseAddress.add(0x{:X});\n    while (checkAddress.readInt() !== {}) {{\n        await new Promise(resolve => setTimeout(resolve, 100));\n    }}\n",
                offset, value
            )
        } else {
            String::new()
        };

        // Build script manually to avoid format string issues with JS {}
        let mut script = format!("// Generated by GraalHax - GS2 Bytecode Injector for {}\n\n", self.client_type.name());

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

        script.push_str("async function injectBytecode(variableName, bytecodeHex) {\n");
        script.push_str(&format!("    var baseAddress = Process.findModuleByName(\"{}\").base;\n", module_name));
        script.push_str(&magic_check);
        script.push_str(&format!("    const TGraalVar_TGraalVar = new NativeFunction(\n"));
        script.push_str(&format!("        baseAddress.add({}), 'void', ['pointer', 'pointer']{});\n", constructor_offset, thiscall_suffix));
        script.push_str(&format!("    const TGraalVar_SetScript = new NativeFunction(\n"));
        script.push_str(&format!("        baseAddress.add({}), 'void', ['pointer', 'pointer']{});\n", setscript_offset, thiscall_suffix));
        script.push_str("    console.log(\"Allocating variable memory...\");\n");
        script.push_str("    const variable = Memory.alloc(0x1000);\n");
        script.push_str("\n");
        script.push_str("    console.log(\"Creating TGraalVar with name: \" + variableName);\n");
        script.push_str("    TGraalVar_TGraalVar(variable, create_tstring(variableName));\n");
        script.push_str("\n");
        script.push_str("    console.log(\"Setting script bytecode (\" + bytecodeHex.length / 3 + \" bytes)...\");\n");
        script.push_str("    TGraalVar_SetScript(variable, create_tstring(hexToBytecode(bytecodeHex)));\n");
        script.push_str("\n");
        script.push_str("    console.log(\"SCRIPT INJECTED successfully\");\n");
        script.push_str("    return true;\n");
        script.push_str("}\n\n");

        script.push_str("// Auto-inject on script load\n");
        script.push_str(&format!("injectBytecode('{}', \"{}\").catch(err => {{\n", var_name_escaped, bytecode_hex));
        script.push_str("    console.log(\"Injection error: \" + err);\n");
        script.push_str("});\n");

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
