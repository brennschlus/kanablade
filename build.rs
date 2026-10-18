fn main() {
    // Explorer and the taskbar read the icon and the "Details" tab from resources in the .exe.
    // Resource id 1 is what main.rs loads as the window icon.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=packaging/icons/kanablade.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("packaging/icons/kanablade.ico", "1")
            .set("ProductName", "kanablade")
            .set("FileDescription", "kanablade · 禅")
            .set("InternalName", "kanablade");
        res.compile().expect("failed to embed Windows resources");
    }
}
