use crate::{link, mapsim, play, records};

pub fn check() {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], link::service_port()));
    let status = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(1)).ok().and_then(|stream| {
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).ok()?;
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(stream), &mut line).ok()?;
        Some(line.trim_end().to_owned())
    });
    println!("controller service on {addr}: {}", status.as_deref().unwrap_or("not answering"));
    println!("bundled service: {}", link::service_program().display());
    let maps = mapsim::maps_folders(&records::default_folders());
    println!("maps folders: {maps:?}");
    match play::current_map(&maps) {
        None => println!("play: no Smashcraft map in Maps/00-Smashcraft"),
        Some(map) => match play::launch_for(&map, std::env::var_os("WARCRAFT_III"), cfg!(windows)) {
            None => println!("play: {} (Warcraft III not found)", map.display()),
            Some(launch) => println!("play: {:?} {:?} prefix {:?}", launch.program, launch.args, launch.prefix),
        },
    }
}
