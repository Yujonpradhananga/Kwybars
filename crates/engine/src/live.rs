use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kwybars_common::config::{VisualizerBackend, VisualizerConfig};
use kwybars_common::spectrum::SpectrumFrame;

use crate::pipeline::{DummySineSource, FrameSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Cava,
    Dummy,
}

pub struct LiveFrameStream {
    latest: Arc<Mutex<SpectrumFrame>>,
    source_kind: SourceKind,
}

impl LiveFrameStream {
    pub fn spawn(config: VisualizerConfig) -> Self {
        let bar_count = config.bars.max(1);
        let latest = Arc::new(Mutex::new(SpectrumFrame::new(
            vec![0.0; bar_count],
            now_millis(),
        )));

        match config.backend {
            VisualizerBackend::Dummy => {
                spawn_dummy_thread(Arc::clone(&latest), bar_count, config.framerate.max(1));
                Self {
                    latest,
                    source_kind: SourceKind::Dummy,
                }
            }
            VisualizerBackend::Auto | VisualizerBackend::Cava => {
                if spawn_cava_thread(Arc::clone(&latest), bar_count, config.framerate.max(1))
                    .is_ok()
                {
                    Self {
                        latest,
                        source_kind: SourceKind::Cava,
                    }
                } else {
                    eprintln!("kwybars: falling back to dummy frame source");
                    spawn_dummy_thread(Arc::clone(&latest), bar_count, config.framerate.max(1));
                    Self {
                        latest,
                        source_kind: SourceKind::Dummy,
                    }
                }
            }
        }
    }

    pub fn source_kind(&self) -> SourceKind {
        self.source_kind
    }

    pub fn latest_frame(&self) -> SpectrumFrame {
        match self.latest.lock() {
            Ok(frame) => frame.clone(),
            Err(_) => SpectrumFrame::new(Vec::new(), now_millis()),
        }
    }
}

fn spawn_dummy_thread(latest: Arc<Mutex<SpectrumFrame>>, bar_count: usize, framerate: u32) {
    let frame_delay = Duration::from_millis((1000_u64 / u64::from(framerate)).max(1));

    thread::spawn(move || {
        let mut source = DummySineSource::new(bar_count);
        loop {
            let frame = source.next_frame();
            if let Ok(mut target) = latest.lock() {
                *target = frame;
            }
            thread::sleep(frame_delay);
        }
    });
}

fn spawn_cava_thread(
    latest: Arc<Mutex<SpectrumFrame>>,
    bar_count: usize,
    framerate: u32,
) -> std::io::Result<()> {
    let config_path = write_cava_config(bar_count, framerate)?;

    thread::spawn(move || {
        let mut command = Command::new("cava");
        command
            .arg("-p")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                eprintln!("kwybars: failed to start cava: {err}");
                let _ = fs::remove_file(&config_path);
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                eprintln!("kwybars: cava did not provide stdout");
                let _ = fs::remove_file(&config_path);
                let _ = child.kill();
                return;
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(bars) = parse_cava_line(&line, bar_count) {
                        let frame = SpectrumFrame::new(bars, now_millis());
                        if let Ok(mut target) = latest.lock() {
                            *target = frame;
                        }
                    }
                }
                Err(err) => {
                    eprintln!("kwybars: error reading cava output: {err}");
                    break;
                }
            }
        }

        let _ = fs::remove_file(&config_path);
        let _ = child.kill();
    });

    Ok(())
}

fn write_cava_config(bar_count: usize, framerate: u32) -> std::io::Result<PathBuf> {
    let timestamp = now_millis();
    let path = env::temp_dir().join(format!(
        "kwybars-cava-{}-{timestamp}.conf",
        std::process::id()
    ));

    let config = format!(
        "[general]
bars = {bar_count}
framerate = {framerate}

[input]
method = pulse
source = auto

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 1000
bar_delimiter = 59
frame_delimiter = 10
"
    );

    fs::write(&path, config)?;
    Ok(path)
}

fn parse_cava_line(line: &str, expected_bars: usize) -> Option<Vec<f32>> {
    let mut bars = Vec::with_capacity(expected_bars);

    for token in line.trim().split(';') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw = match trimmed.parse::<f32>() {
            Ok(value) => value,
            Err(_) => return None,
        };
        bars.push((raw / 1000.0).clamp(0.0, 1.0));
    }

    if bars.is_empty() {
        return None;
    }

    if bars.len() > expected_bars {
        bars.truncate(expected_bars);
    } else if bars.len() < expected_bars {
        bars.resize(expected_bars, 0.0);
    }

    Some(bars)
}

fn now_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().min(u64::MAX as u128) as u64,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_cava_line;

    #[test]
    fn parses_ascii_bar_line() {
        let parsed = parse_cava_line("50;125;1000\n", 3);
        assert_eq!(parsed, Some(vec![0.05, 0.125, 1.0]));
    }

    #[test]
    fn pads_short_line_to_expected_count() {
        let parsed = parse_cava_line("900;450\n", 4);
        assert_eq!(parsed, Some(vec![0.9, 0.45, 0.0, 0.0]));
    }
}
