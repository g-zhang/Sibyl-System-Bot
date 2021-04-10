use std::io;
#[cfg(windows)]
use winres::WindowsResource;

fn main() -> io::Result<()> {
    #[cfg(windows)]
    {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/app.ico")
            .set("ProductName", "Sibyl System")
            .set("FileDescription", "Sibyl System Discord Bot Server Process")
            .set("CompanyName", "gordonz.dev")
            .set_manifest(
                r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
               <ms_asmv2:trustInfo xmlns:ms_asmv2="urn:schemas-microsoft-com:asm.v2">
                  <ms_asmv2:security>
                     <ms_asmv2:requestedPrivileges>
                        <ms_asmv2:requestedExecutionLevel level="asInvoker" uiAccess="false">
                        </ms_asmv2:requestedExecutionLevel>
                     </ms_asmv2:requestedPrivileges>
                  </ms_asmv2:security>
               </ms_asmv2:trustInfo>
            </assembly>
            "#,
            )
            .compile()?;
    }
    Ok(())
}
