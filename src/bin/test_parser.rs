use std::fs;
#[path = "../ifc_converter.rs"]
mod ifc_converter;

fn main() {
    let text = fs::read_to_string("/home/hr/Ralltech/clients/gamerdinger/mounts/network_folder/Ablage/OrthoGraph Tool/Project_1.ifc").unwrap();
    let res = ifc_converter::convert_ifc(&text);
    match res {
        Ok((out, rep)) => {
            println!("SUCCESS: Repaired {} openings.", rep);
            fs::write("/tmp/Project_1_Allplan.ifc", out).unwrap();
        },
        Err(e) => println!("ERROR: {}", e),
    }
}
