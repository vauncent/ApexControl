fn main(){
    println!("cargo:rerun-if-changed=assets/apexcontrol.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref()==Ok("windows") {
        winresource::WindowsResource::new().set_icon("assets/apexcontrol.ico")
            .set("ProductName","ApexControl").set("FileDescription","ApexControl keyboard control")
            .compile().expect("compile Windows icon resource");
    }
}
