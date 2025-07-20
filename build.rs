fn main() {
  #[cfg(target_os = "windows")]
  {
    let mut res = winres::WindowsResource::new();
    res.set_icon("./res/icon.ico");
    res.compile().unwrap();
    println!("cargo:rerun-if-changed=./res/icon.ico");
  }

  #[cfg(target_os = "linux")]
  {
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rerun-if-changed=./res/icon.png");
  }

  #[cfg(target_os = "macos")]
  {
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    println!("cargo:rerun-if-changed=./res/icon.icns");
  }

  #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
  {
    println!(
      "cargo:warning=Building on an unsupported platform. Some features may not work correctly."
    );
  }
}
