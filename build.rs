//! Windows only: embed product version info (VERSIONINFO) and manifest into `open-cleaner.exe`.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "OpenCleaner")
            .set("FileDescription", "OpenCleaner System Privacy & Cleaning Suite")
            .set("CompanyName", "OpenCleaner Open Source")
            .set("LegalCopyright", "Copyright (c) 2026 shafo3i. MIT OR Apache-2.0.")
            .set("OriginalFilename", "open-cleaner.exe")
            .set("InternalName", "open-cleaner");

        // Application manifest declaring standard execution level (asInvoker)
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="asInvoker" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
<compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
        <!-- Windows 10 and Windows 11 -->
        <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
</compatibility>
</assembly>
"#);

        if let Err(e) = res.compile() {
            println!("cargo:warning=embedding Windows resources failed: {e}");
        }
    }
}
