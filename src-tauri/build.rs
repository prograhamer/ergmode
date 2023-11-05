fn main() {
    ant_network_key_from_env();
    tauri_build::build()
}

fn ant_network_key_from_env() {
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR must be set");

    let network_key = std::env::var("ANT_NETWORK_KEY").expect("ANT_NETWORK_KEY must be set");
    let network_key: [u8; 8] = hex::decode(network_key)
        .expect("ANT_NETWORK_KEY must be valid hex")
        .try_into()
        .expect("ANT_NETWORK_KEY must be hex representation of 8 bytes");

    let network_key = network_key.map(|e| e.to_string()).join(", ");

    let dest_path = std::path::Path::new(&out_dir).join("ant_network_key.rs");
    std::fs::write(
        dest_path,
        format!("const ANT_NETWORK_KEY : [u8; 8] = [{}];", network_key),
    )
    .unwrap();
}
