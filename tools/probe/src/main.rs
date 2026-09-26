//! Standalone HID probe: lists every device hidapi can see, with paths, so we
//! can compare against Windows Device Manager / PowerShell output.
//! Run: cargo run --manifest-path tools/probe/Cargo.toml

fn main() {
    let api = match hidapi::HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            println!("FATAL: hid init failed: {}", e);
            return;
        }
    };

    let mut total = 0usize;
    let mut cm: Vec<String> = Vec::new();
    for d in api.device_list() {
        total += 1;
        let path = String::from_utf8_lossy(d.path().to_bytes_with_nul())
            .trim_end_matches('\0')
            .to_string();
        let is_cm = d.vendor_id() == 0x2516 || d.vendor_id() == 0x2512;
        if is_cm {
            cm.push(format!(
                "{:04x}:{:04x} if={} up=0x{:04x} usage=0x{:04x} path={}",
                d.vendor_id(),
                d.product_id(),
                d.interface_number(),
                d.usage_page(),
                d.usage(),
                path
            ));
        }
        println!(
            "{:04x}:{:04x}  if={:<3} up=0x{:04x} usage=0x{:04x}  product={:?} mfr={:?}",
            d.vendor_id(),
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            d.product_string().unwrap_or(""),
            d.manufacturer_string().unwrap_or(""),
        );
        println!("    path: {}", path);
    }

    println!("\n=== total hid devices: {} ===", total);
    println!("=== Cooler Master devices: {} ===", cm.len());
    for line in &cm {
        println!("  {}", line);
    }
}
