// Use Frida's Memory.alloc for memory allocation
const alloc_mem = (size) => Memory.alloc(size);
const malloc = new NativeFunction(Module.getGlobalExportByName("malloc"), "pointer", ["int"]);

function create_tstring(e) {
    // Use Memory.alloc directly for cleaner allocation
    const r = Memory.alloc(8);
    const a = Memory.alloc(e.length + 8 + 10);
    r.writePointer(a);
    a.writeInt(e.length);
    ptr(parseInt(a) + 4).writeInt(100);

    for (let i = 0; i < e.length; i++) {
        ptr(parseInt(a) + 8).add(i).writeU8(e.charCodeAt(i));
    }
    ptr(parseInt(a) + 8).add(e.length).writeU8(0);
    return r;
}
const hexToBytecode = (hex) => { return hex.split(" ").map(byte => String.fromCharCode(parseInt(byte, 16))).join(""); }
async function init() {
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    var baseAddress = Process.findModuleByName("Graal.exe").base;
    var check_address = baseAddress.add(0x17da90);
    while (check_address.readInt() != 157876074) {
        await sleep(100);
    }

    const TGraalVar_TGraalVar = new NativeFunction(baseAddress.add(0x195770), 'void', ['pointer', 'pointer'], "thiscall");
    const TGraalVar_SetScript = new NativeFunction(baseAddress.add(0x196290), 'void', ['pointer', 'pointer'], "thiscall")

    console.log("Waiting 10 seconds before injecting script...");
    await sleep(10000);  
    console.log("Injecting script...");
    const variable = alloc_mem(0x1000);
    console.log("Allocated memory for variable");
    TGraalVar_TGraalVar(variable, create_tstring("VarName"));
    console.log("Created TGraalVar instance");
    TGraalVar_SetScript(variable, create_tstring(hexToBytecode("00 00 00 01 00 00 00 04 00 00 00 00 00 00 00 02 00 00 00 29 00 00 00 01 6F 6E 43 72 65 61 74 65 64 00 00 00 00 14 47 72 61 61 6C 43 6F 6E 74 72 6F 6C 2E 6F 6E 4B 65 79 44 6F 77 6E 00 00 00 00 03 00 00 00 0F 63 68 61 74 00 48 69 00 70 61 72 61 6D 73 00 00 00 00 04 00 00 00 37 01 F4 00 1F 17 33 0A 09 B6 16 F0 00 23 15 F0 01 32 14 F3 1E 08 B6 16 F0 00 23 15 F0 01 32 14 F3 00 07 01 F4 00 1F 17 33 0A B6 16 F0 00 23 16 F0 02 32 14 F3 00 07 07 0A 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00")));
    console.log("SCRIPT INJECTED");
}



init();