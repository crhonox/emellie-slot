#[cfg(not(feature = "host"))]
mod device_app;
#[cfg(feature = "host")]
mod host_app;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--init-root") => {
            let Some(dir) = args.next() else {
                eprintln!("omega-zero: --init-root needs a directory");
                std::process::exit(2);
            };
            omega_zero::root::ensure(std::path::Path::new(&dir));
            return;
        }
        Some("--volume-keys") => {
            let Some(dir) = args.next() else {
                eprintln!("omega-zero: --volume-keys needs the card");
                std::process::exit(2);
            };
            omega_zero::volume_keys::run(std::path::Path::new(&dir));
            return;
        }
        Some("--progress") => {
            let (Some(dir), Some(title)) = (args.next(), args.next()) else {
                eprintln!("omega-zero: --progress needs the card and a title");
                std::process::exit(2);
            };
            omega_zero::progress::run(std::path::Path::new(&dir), &title);
            return;
        }
        _ => {}
    }

    #[cfg(feature = "host")]
    host_app::run();

    #[cfg(not(feature = "host"))]
    device_app::run();
}
