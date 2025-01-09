use std::error::Error;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

const DIGEST_FILE_NAME: &str = "about.Cargo.lock.digest";
const ABOUT_FILE_NAME: &str = "about.html";
const ABOUT_CONFIG: &str = "cargo-about.toml";
const ABOUT_TEMPLATE: &str = "cargo-about.hbs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_about_html()
}

fn generate_about_html() -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let about_path = PathBuf::from(&out_dir).join(ABOUT_FILE_NAME);
    let digest_path = PathBuf::from(&out_dir).join(DIGEST_FILE_NAME);

    let last_digest = read_last_cargo_lock_digest(&digest_path)?;
    let digest = compute_cargo_lock_digest()?;

    if !&about_path.try_exists()? || last_digest != digest {
        exec_about_generator(&about_path)?;
        std::fs::write(digest_path, digest)?;
    }

    println!("cargo:rustc-env=ABOUT_HTML_PATH={}", &about_path.canonicalize()?.display());

    Ok(())
}

fn compute_cargo_lock_digest() -> Result<String, Box<dyn Error>> {
    let cargo_lock = std::fs::read_to_string("Cargo.lock")?;
    Ok(sha256::digest(cargo_lock))
}

fn read_last_cargo_lock_digest<P>(path: P) -> Result<String, std::io::Error>
where
    P: AsRef<Path>,
{
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn exec_about_generator<P>(path: P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    if let Some(p) = path.as_ref().parent() {
        std::fs::create_dir_all(p)?;
    }

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("about");
    cmd.arg("generate");
    cmd.arg("--fail");
    cmd.arg("--config");
    cmd.arg(ABOUT_CONFIG);
    cmd.arg("--output-file");
    cmd.arg(path.as_ref());
    cmd.arg(ABOUT_TEMPLATE);

    let status = cmd.status()?;
    if !status.success() {
        Err(format!("{status}"))?;
    }

    Ok(())
}
