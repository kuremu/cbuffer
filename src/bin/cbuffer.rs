use cbuffer::{record, state_is, Action, State, StateMut, BUFSIZE};
use clap::Parser;
use raw_tty::TtyWithGuard;
use std::io::prelude::Read;
use std::io::{stderr, stdin, stdout, Result, Write};
use std::sync::{Arc, Mutex};
use std::{fs, thread, time};

const SEGLEN: usize = 10;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value_t = 1411200)]
    bitrate: usize,
    #[clap(default_value_t = 5)]
    seconds: usize,
}

macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

fn main() -> Result<()> {
    let args = Args::parse();
    if atty::is(atty::Stream::Stdin) {
        eprintln!("Error: stdin not redirected");
        std::process::exit(1);
    }
    if args.bitrate < BUFSIZE * 8 {
        eprintln!("Error: bitrate must be greater than {}", BUFSIZE);
        std::process::exit(1);
    }
    if args.seconds == 1 {
        eprintln!("Error: buffer length must be greater than 1s");
        std::process::exit(1);
    }
    let len = (args.bitrate as f64 / 8f64 * args.seconds as f64 / BUFSIZE as f64).ceil() as usize;
    let state = Arc::new(Mutex::new(State {
        action: Action::Buffer,
        buffered: 0,
        written: 0,
        bitrate: args.bitrate,
        seconds: args.seconds,
    }));
    let stdin = stdin();
    let stdout = stdout();
    let stderr = stderr();

    let tty = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    // spawning extra TtyWithGuard to restore later in case of panic during read_input
    let raw_tty = TtyWithGuard::new(tty)?;
    let raw_tty_2 = raw_tty.try_clone()?;
    let t_input = thread::spawn(enclose! { (state) move || { read_input(state, raw_tty_2) } });
    let t_ui = thread::spawn(enclose! { (state) move || { ui_loop(state, stderr) } });

    if let Err(e) = record(stdin, stdout, len, &state) {
        if let Ok(mut state) = state.lock() {
            eprint!("\rError: {}\n\r", e);
            state.action = Action::Finish;
            drop(t_input);
            drop(t_ui);
            std::process::exit(1);
        }
    } else {
        let _ = t_input.join();
        let _ = t_ui.join();
    }

    Ok(())
}

fn read_input(state: StateMut, tty: fs::File) -> Result<()> {
    let mut res = [0; 1];
    let mut raw_tty = TtyWithGuard::new(tty)?;
    raw_tty.set_raw_mode()?;
    while !state_is(&state, Action::Finish) {
        thread::sleep(time::Duration::from_millis(10));
        if raw_tty.read(&mut res)? > 0 {
            if let Ok(mut state) = state.lock() {
                state.action = match_char(res[0], &state.action);
            }
        }
    }
    Ok(())
}

fn match_char(c: u8, action: &Action) -> Action {
    match c {
        0x03 | 0x71 | 0x51 => Action::Finish,
        _ => match action {
            Action::Record => Action::Buffer,
            _ => Action::Record,
        },
    }
}

fn ui_loop<U: Write>(state: StateMut, mut writer: U) -> Result<()> {
    while !state_is(&state, Action::Finish) {
        let _ = write_ui(&state, &mut writer);
        thread::sleep(time::Duration::from_millis(100));
    }
    let _ = writer.write(b"\n\n")?;
    writer.flush()?;
    Ok(())
}

fn write_ui<U: Write>(state: &StateMut, writer: &mut U) -> Result<()> {
    if let Ok(state) = state.lock() {
        let status = match state.action {
            Action::Record => "\x1b[31mrec\x1b[m",
            _ => "pre",
        };
        let bar = get_bar_str(&state);
        let size = get_size_str(state.written);
        let time = get_time_str(state.written, state.bitrate / 8);
        let ui = format!(
            "\x1b[1K\r{} {}\x1b[K\n\r{}  {}\x1b[K\x1b[1A\r",
            status, bar, time, size,
        );
        let _ = writer.write(&ui.into_bytes())?;
        writer.flush()?;
    }
    Ok(())
}

fn get_bar_str(state: &State) -> String {
    let buffered: f64 = state.buffered as f64;
    let capacity: f64 = (state.bitrate / 8 * state.seconds) as f64;
    let perc = buffered / capacity;
    let segments = (SEGLEN as f64 * perc) as usize;
    let buffered = (state.seconds as f64 * perc) as usize;
    format!(
        "<{}{}> {}/{}s",
        str::repeat("=", segments),
        str::repeat("-", SEGLEN - segments),
        buffered,
        state.seconds
    )
}

fn get_size_str(b: usize) -> String {
    let kb = b / 1000;
    let mb = kb / 1000;
    if mb > 0 {
        format!("{}.{} MB", mb, ((kb as f64 % 1e3) / 1e3 * 10f64) as usize)
    } else if kb > 0 {
        format!("{} KB", kb)
    } else {
        format!("{} B", b)
    }
}

fn get_time_str(written: usize, rate: usize) -> String {
    let seconds = written / rate;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes % 60, seconds % 60)
}
