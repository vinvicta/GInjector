use frida_bridge::FridaInjector;
use frida_bridge::ClientType;

fn main() {
    let injector = FridaInjector::new(ClientType::EraSteam);
    let script = injector.generate_injection_script("00 01 02", ".");

    println!("{}", script);
}
