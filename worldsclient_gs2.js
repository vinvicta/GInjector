const malloc = new NativeFunction(Module.findExportByName(null, "malloc"), "pointer", ["int"]);
 
function create_tstring(e) {

        const r = malloc(8);
        const a = malloc(e.length + 8 + 10); 
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
  function init() {

  
    
    var baseAddress = Process.findModuleByName("Graal3DEngine.dll").base;
    
     
  

    

    const TGraalVar_TGraalVar = new NativeFunction(baseAddress.add(0x7FF901F32340-0x7FF901D90000), 'void', ['pointer', 'pointer']);
    const TGraalVar_SetScript = new NativeFunction(baseAddress.add(0x7FF901F37DE0-0x7FF901D90000), 'void', ['pointer', 'pointer'])

 
   
    console.log("Waiting 10 seconds before injecting script...");
  
    console.log("Injecting script...");
    const variable =malloc(0x1000);
    console.log("Allocated memory for variable");
    TGraalVar_TGraalVar(variable, create_tstring("."));
    console.log("Created TGraalVar instance");
    TGraalVar_SetScript(variable, create_tstring(hexToBytecode("00 00 00 01 00 00 00 04 00 00 00 00 00 00 00 02 00 00 00 0E 00 00 00 01 6F 6E 43 72 65 61 74 65 64 00 00 00 00 03 00 00 00 0E 63 68 61 74 00 49 6E 6A 65 63 74 65 64 00 00 00 00 04 00 00 00 15 01 F4 00 0C 17 33 0A B6 16 F0 00 23 15 F0 01 32 14 F3 00 07 07 0A 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00")));

    console.log("SCRIPT INJECTED");
}



init();
