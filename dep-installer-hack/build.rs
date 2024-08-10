fn main() {
    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        if !std::process::Command::new("sh")
            .arg("-c")
            .arg("apt-get update && apt-get install -y build-essential libopencv-dev cmake pkg-config")
            .status()
            .expect("failed to run apt")
            .success()
        {
            panic!("failed to install dependencies")
        }
    }
}
