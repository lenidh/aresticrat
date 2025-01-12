use crate::print_log;
use crate::verbosity;
use crate::DEFAULT_VERBOSITY;
use std::borrow::BorrowMut;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::thread::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::Level;
use tracing_subscriber::fmt::writer::EitherWriter;

pub fn run_sequential<C, I>(
    cmds: I,
    quiet: bool,
) -> Result<std::process::ExitStatus, std::io::Error>
where
    I: IntoIterator<Item = C>,
    C: BorrowMut<Command>,
{
    for mut cmd in cmds {
        let status = run(cmd.borrow_mut(), quiet)?;
        if !status.success() {
            return Ok(status);
        }
    }
    Ok(Default::default())
}

pub fn run(cmd: &mut Command, quiet: bool) -> Result<std::process::ExitStatus, std::io::Error> {
    let print = !quiet && verbosity() >= DEFAULT_VERBOSITY;

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    print_log!(Level::DEBUG, "Run command: {cmd:?} ...");
    let mut child = cmd.spawn()?;
    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let out_task = spawn_tee(child_stdout, filter_writer(print, std::io::stdout()));
    let err_task = spawn_tee(child_stderr, filter_writer(print, std::io::stderr()));

    let status = child.wait()?;
    let out = out_task.join().unwrap()?;
    let err = err_task.join().unwrap()?;

    log_cmd_result(cmd, &status, &out, &err, quiet);

    Ok(status)
}

fn log_cmd_result(
    cmd: &std::process::Command,
    status: &std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
    quiet: bool,
) {
    let out = String::from_utf8_lossy(stdout);
    let err = String::from_utf8_lossy(stderr);

    let mut str = String::new();
    str.push_str(&format!("Finished command {cmd:?}\nStatus: {status}"));

    if !out.is_empty() {
        if quiet {
            debug!("Stdout:\n{out}");
        } else {
            info!("{out}");
        }
    }
    if !err.is_empty() {
        if quiet {
            debug!("Stdout:\n{out}");
        } else {
            error!("{err}");
        }
    }

    debug!("Command completed. (Status: {status})");
}

pub struct Tee<R: Read, W: Write>(R, W);

impl<R: Read, W: Write> Tee<R, W> {
    pub fn new(r: R, w: W) -> Self {
        Self(r, w)
    }
}

impl<R: Read, W: Write> Read for Tee<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            match self.0.read(buf) {
                Ok(0) => return Ok(0),
                Ok(n) => {
                    self.1.write_all(&buf[..n])?;
                    return Ok(n);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            };
        }
    }
}

fn spawn_tee<R: 'static + Read + Send, W: 'static + Write + Send>(
    r: R,
    w: W,
) -> JoinHandle<Result<Vec<u8>, std::io::Error>> {
    std::thread::spawn(move || tee_all(r, w))
}

fn tee_all<R: Read, W: Write>(r: R, w: W) -> Result<Vec<u8>, std::io::Error> {
    let mut v = Vec::new();
    let mut t = Tee::new(r, w);
    t.read_to_end(&mut v)?;
    Ok(v)
}

fn filter_writer<W: Write>(condition: bool, w: W) -> impl Write {
    if condition {
        EitherWriter::A(w)
    } else {
        EitherWriter::B(std::io::sink())
    }
}
