use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
    time::{Duration, SystemTime},
};

use clap::Parser;
use espflash::{flasher::Flasher, interface::Interface};
use serialport::{SerialPortInfo, SerialPortType};

#[derive(Parser)]
struct Cli {
    /// Path to ESP32 elf files
    #[arg(long)]
    esp32: Option<PathBuf>,

    /// Path to ESP32-S2 elf files
    #[arg(long)]
    esp32s2: Option<PathBuf>,

    /// Path to ESP32-S3 elf files
    #[arg(long)]
    esp32s3: Option<PathBuf>,

    /// Path to ESP32-C2 elf files
    #[arg(long)]
    esp32c2: Option<PathBuf>,

    /// Path to ESP32-C3 elf files
    #[arg(long)]
    esp32c3: Option<PathBuf>,

    /// Path to ESP32-C6 elf files
    #[arg(long)]
    esp32c6: Option<PathBuf>,

    /// Path to ESP32-H2 elf files
    #[arg(long)]
    esp32h2: Option<PathBuf>,
}

fn main() {
    pretty_env_logger::init();
    let cli = Cli::parse();

    let mut boards = Vec::new();
    let mut serial_ports = Vec::new();

    let ports = serialport::available_ports().unwrap();
    for port in ports {
        let SerialPortType::UsbPort(info) = port.clone().port_type else {
            panic!();
        };
        let mut flasher = Flasher::connect(
            Interface::new(&port, None, None).unwrap(),
            info,
            Some(921600),
            true,
        )
        .unwrap();

        let binfo = flasher.device_info().unwrap();
        boards.push(binfo.chip);
        serial_ports.push(port);
    }

    let mut test_paths: Vec<(espflash::targets::Chip, PathBuf)> = Vec::new();
    if let Some(path) = cli.esp32 {
        test_paths.push((espflash::targets::Chip::Esp32, path));
    }
    if let Some(path) = cli.esp32s2 {
        test_paths.push((espflash::targets::Chip::Esp32s2, path));
    }
    if let Some(path) = cli.esp32s3 {
        test_paths.push((espflash::targets::Chip::Esp32s3, path));
    }
    if let Some(path) = cli.esp32c2 {
        test_paths.push((espflash::targets::Chip::Esp32c2, path));
    }
    if let Some(path) = cli.esp32c3 {
        test_paths.push((espflash::targets::Chip::Esp32c3, path));
    }
    if let Some(path) = cli.esp32c6 {
        test_paths.push((espflash::targets::Chip::Esp32c6, path));
    }
    if let Some(path) = cli.esp32h2 {
        test_paths.push((espflash::targets::Chip::Esp32h2, path));
    }

    for (chip, _) in &test_paths {
        run_all_tests_for_chip(*chip, &test_paths, &serial_ports, &boards);
    }
}

fn run_all_tests_for_chip(
    chip: espflash::targets::Chip,
    paths: &Vec<(espflash::targets::Chip, PathBuf)>,
    ports: &Vec<SerialPortInfo>,
    boards: &Vec<espflash::targets::Chip>,
) {
    run_tests_for_chip_internal(chip, paths, ports, boards, None);
}

fn run_tests_for_chip_internal(
    chip: espflash::targets::Chip,
    paths: &Vec<(espflash::targets::Chip, PathBuf)>,
    ports: &Vec<SerialPortInfo>,
    boards: &Vec<espflash::targets::Chip>,
    specific_executable: Option<String>,
) {
    if specific_executable.is_none() {
        println!();
        println!("Running tests on {}", chip);
    }

    let index = boards
        .into_iter()
        .enumerate()
        .find(|(_, c)| **c == chip)
        .unwrap()
        .0;
    let port = &ports[index];

    let path = &paths.into_iter().find(|(c, _)| *c == chip).unwrap().1;

    // collect elf files
    // only files starting with `test` will be used unless `specific_executable`
    // is given - in that case it must exactly match. ELFs are not allowed to contain `-` in their name.
    let tests: Vec<(Vec<u8>, String)> = path
        .read_dir()
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| (e.path(), e.file_name()))
        .filter(|(_, n)| !n.to_str().unwrap().contains("-"))
        .filter(|(_, n)| {
            specific_executable.is_none() && n.to_str().unwrap().starts_with("test")
                || specific_executable.is_some()
                    && n.to_str().unwrap() == specific_executable.as_ref().unwrap()
        })
        .map(|(f, n)| {
            let bytes = std::fs::read(f).unwrap();
            (bytes, n.to_str().unwrap().to_string())
        })
        .filter(|(b, _n)| {
            b.len() > 4 && b[0] == 0x7f && b[1] == b'E' && b[2] == b'L' && b[3] == b'F'
        })
        .collect();

    // flash and run each file
    for (elf, filename) in tests {
        println!("{filename}");

        let SerialPortType::UsbPort(info) = port.clone().port_type else {
            panic!();
        };
        let mut con = Flasher::connect(
            Interface::new(&port, None, None).unwrap(),
            info,
            Some(921600),
            true,
        )
        .unwrap();

        con.change_baud(921600).unwrap();
        con.load_elf_to_flash(&elf, None, None, None, None, None, None)
            .unwrap();

        if specific_executable.is_some() {
            // if a `specific_executable` is given that should just run - we don't want to check anything
            break;
        }

        let mut serial = con.into_interface().serial_port;

        let started_at = SystemTime::now();
        serial.set_baud_rate(115200).unwrap();
        serial.set_timeout(Duration::from_millis(5)).unwrap();
        let reader = BufReader::new(serial);
        let mut reader = reader.lines();
        loop {
            if let Ok(s) = reader.next().unwrap() {
                log::debug!("DEVICE: {}", s);

                if s.starts_with("[PASSED]") {
                    println!("{filename} => {s}");
                    break;
                }
                if s.starts_with("[FAILED") {
                    println!("{filename} => {s}");
                    break;
                }
                if s.starts_with("[HOST ") {
                    let cmd = s.strip_prefix("[HOST ").unwrap().strip_suffix("]").unwrap();
                    run_cmd(cmd);
                }
                if s.starts_with("[RUN ") {
                    let cmd = s.strip_prefix("[RUN ").unwrap().strip_suffix("]").unwrap();
                    run_on_chip(cmd, paths, ports, boards);
                }
            }

            if started_at.elapsed().unwrap() > Duration::from_secs(20) {
                println!("{filename} => TIMEOUT");
                break;
            }
        }
    }
}

fn run_on_chip(
    cmd: &str,
    paths: &Vec<(espflash::targets::Chip, PathBuf)>,
    ports: &Vec<SerialPortInfo>,
    boards: &Vec<espflash::targets::Chip>,
) {
    let parts: Vec<&str> = cmd.split(" ").collect();
    log::debug!("Flashing {:?}", &parts);
    run_tests_for_chip_internal(
        espflash::targets::Chip::try_from(parts[0]).unwrap(),
        paths,
        ports,
        boards,
        Some(parts[1].to_string()),
    );
}

// simply run a command on the host - failing or not doesn't change anything to the test result
fn run_cmd(cmd: &str) {
    let parts: Vec<&str> = cmd.split(" ").collect();
    log::debug!("running command {:?}", &parts);
    let output = Command::new(parts[0]).args(&parts[1..]).output().unwrap();
    log::debug!("raw command output {:02x?}", &output.stdout);
    log::debug!("command output {}", unsafe {
        std::str::from_utf8_unchecked(&output.stdout)
    });
}
