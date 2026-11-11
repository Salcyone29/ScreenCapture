//! Sonde : FFmpeg accepte-t-il de couper puis rétablir le son du micro pendant un enregistrement
//! (commande « c » sur l'entrée standard, filtre `volume`) ? Analyse ensuite le volume par tranche.
//! Usage : `cargo run --release --example mute_probe -- <ffmpeg.exe> "<micro>" <sortie.mp4>`

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (ffmpeg, mic, out) = (&args[1], &args[2], &args[3]);
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "gdigrab", "-framerate", "15", "-video_size", "320x240", "-i", "desktop"])
        .args(["-f", "dshow", "-i", &format!("audio={mic}")])
        .args(["-filter:a", "volume=volume=1:eval=frame", "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", out])
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg");
    let mut stdin = child.stdin.take().unwrap();
    std::thread::sleep(Duration::from_secs(3));
    stdin.write_all(b"cvolume -1 volume 0\n").unwrap();
    stdin.flush().unwrap();
    println!("coupé à 3 s");
    std::thread::sleep(Duration::from_secs(3));
    stdin.write_all(b"cvolume -1 volume 1\n").unwrap();
    stdin.flush().unwrap();
    println!("rétabli à 6 s");
    std::thread::sleep(Duration::from_secs(3));
    stdin.write_all(b"q").unwrap();
    drop(stdin);
    let _ = child.wait();
    for k in 0..18 {
        let (from, to) = (k as f32 * 0.5, k as f32 * 0.5 + 0.5);
        let o = Command::new(ffmpeg)
            .args(["-hide_banner", "-ss", &format!("{from}"), "-to", &format!("{to}"), "-i", out, "-vn", "-af", "volumedetect", "-f", "null", "-"])
            .output()
            .unwrap();
        let t = String::from_utf8_lossy(&o.stderr);
        let mean = t.lines().find(|l| l.contains("mean_volume")).unwrap_or("?");
        let max = t.lines().find(|l| l.contains("max_volume")).unwrap_or("?");
        println!("{from}-{to} s : {} | {}", mean.split("] ").last().unwrap_or(mean), max.split("] ").last().unwrap_or(max));
    }
}
