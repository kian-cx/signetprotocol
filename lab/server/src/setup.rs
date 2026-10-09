//! First-run setup. No personal paths: everything lands in the data directory
//! (`$SIGNET_HOME`, else the per-system data directory: `~/.local/share/signet` on Linux,
//! `~/Library/Application Support/signet` on macOS, `%LOCALAPPDATA%\signet` on Windows).
//!
//! Server use needs only this binary and a world file.
//! Client use (Link and Forge) additionally needs the decision model and a Python
//! environment. Those are downloaded only when this command is asked to.
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MODEL_NAME: &str = "signet-forge-dm-qwen35-0.8b-clef-distill-v1";
const MODEL_URL: &str = "https://media.githubusercontent.com/media/signetprotocol/signet/main/models/signet-forge-dm-qwen35-0.8b-clef-distill-v1";

const MODEL_FILES: &[&str] = &[
    "LICENSE", "README.md", "chat_template.jinja", "config.json", "generation_config.json",
    "joint_head.safetensors", "joint_head_config.json", "joint_schema_model.py",
    "model.safetensors-00001-of-00001.safetensors", "model.safetensors.index.json",
    "processor_config.json", "student_train_info.json", "tokenizer.json", "tokenizer_config.json",
    "unsloth_decision_config.json",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub role: String,
    pub games: Vec<String>,
    pub model: String,
    pub python: String,
    pub forge: String,
    pub lc_script: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { role: String::new(), games: vec![], model: String::new(), python: String::new(), forge: String::new(), lc_script: String::new() }
    }
}

pub fn data_dir() -> PathBuf {
    data_dir_in(std::env::consts::OS, &std::env::var("SIGNET_HOME").unwrap_or_default(), &std::env::var("XDG_DATA_HOME").unwrap_or_default(),
        &std::env::var("HOME").unwrap_or_default(), &std::env::var("USERPROFILE").unwrap_or_default(), &std::env::var("LOCALAPPDATA").unwrap_or_default())
}

/// Where setup writes, given the OS name and the environment values. Empty strings mean unset.
pub fn data_dir_in(os: &str, signet_home: &str, xdg: &str, home: &str, userprofile: &str, localappdata: &str) -> PathBuf {
    if !signet_home.is_empty() { return PathBuf::from(signet_home); }
    let home = if !home.is_empty() { home } else { userprofile };
    match os {
        "windows" => {
            if !localappdata.is_empty() { return PathBuf::from(localappdata).join("signet"); }
            if !home.is_empty() { return PathBuf::from(home).join("AppData").join("Local").join("signet"); }
        }
        "macos" => {
            if !xdg.is_empty() { return PathBuf::from(xdg).join("signet"); }
            if !home.is_empty() { return PathBuf::from(home).join("Library/Application Support/signet"); }
        }
        _ => {
            if !xdg.is_empty() { return PathBuf::from(xdg).join("signet"); }
            if !home.is_empty() { return PathBuf::from(home).join(".local/share/signet"); }
        }
    }
    std::env::temp_dir().join("signet")
}

fn config_path() -> PathBuf { data_dir().join("config") }

pub fn load() -> Config {
    let mut c = Config::default();
    let Ok(text) = fs::read_to_string(config_path()) else { return c };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match k {
            "role" => c.role = v.to_string(),
            "games" => c.games = v.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
            "model" => c.model = v.to_string(),
            "python" => c.python = v.to_string(),
            "forge" => c.forge = v.to_string(),
            "lc_script" => c.lc_script = v.to_string(),
            _ => {}
        }
    }
    c
}

fn save(c: &Config) -> io::Result<()> {
    fs::create_dir_all(data_dir())?;
    let games = c.games.join(",");
    let text = format!(
        "role={}\ngames={}\nmodel={}\npython={}\nforge={}\nlc_script={}\n",
        c.role, games, c.model, c.python, c.forge, c.lc_script
    );
    fs::write(config_path(), text)
}

fn have(bin: &str) -> bool { which(bin).is_some() }

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let mut names = vec![bin.to_string()];
    if cfg!(windows) {
        for ext in [".exe", ".cmd", ".bat", ".EXE"] {
            names.push(format!("{bin}{ext}"));
        }
    }
    for dir in std::env::split_paths(&path) {
        for name in &names {
            let p = dir.join(name);
            if p.is_file() { return Some(p); }
        }
    }
    None
}

/// `python3` on Linux and macOS, `python` on Windows when that is the name the installer used.
fn python_bin() -> Option<PathBuf> {
    which("python3").or_else(|| which("python"))
}

fn command_out(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin).args(args).output().ok()?;
    if !out.status.success() { return None; }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn is_lfs_pointer(path: &Path) -> bool {
    let Ok(f) = fs::read(path) else { return false };
    f.starts_with(b"version https://git-lfs.github.com/spec/v1")
}

fn model_files() -> Vec<String> {
    if let Ok(list) = std::env::var("SIGNET_MODEL_FILES") {
        if !list.is_empty() { return list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(); }
    }
    MODEL_FILES.iter().map(|s| s.to_string()).collect()
}

fn model_base_url() -> String {
    std::env::var("SIGNET_MODEL_URL").unwrap_or_else(|_| MODEL_URL.into())
}

/// A checkout that already has the real weights (not a Git LFS pointer).
fn bundled_model() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models").join(MODEL_NAME);
    let weights = p.join("model.safetensors-00001-of-00001.safetensors");
    if weights.is_file() && !is_lfs_pointer(&weights) { Some(p) } else { None }
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for name in model_files() {
        let src = from.join(&name);
        if src.is_file() && !is_lfs_pointer(&src) {
            fs::copy(&src, to.join(&name))?;
        }
    }
    Ok(())
}

fn http_get(url: &str, out: &Path) -> Result<(), String> {
    if let Some(curl) = which("curl") {
        let status = Command::new(curl).args(["-fsSL", "--retry", "2", "-o"]).arg(out).arg(url).status().map_err(|e| e.to_string())?;
        if status.success() { return Ok(()); }
        let _ = fs::remove_file(out);
        return Err(format!("download failed: {url}"));
    }
    let python = python_bin().ok_or("neither curl nor Python is installed; one of them is needed to download the model")?;
    let code = "import sys, urllib.request\nurllib.request.urlretrieve(sys.argv[1], sys.argv[2])\n";
    let status = Command::new(python).args(["-c", code, url]).arg(out).status().map_err(|e| e.to_string())?;
    if status.success() { return Ok(()); }
    let _ = fs::remove_file(out);
    Err(format!("download failed: {url}"))
}

fn download_model(dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let base = model_base_url().trim_end_matches('/').to_string();
    for name in model_files() {
        let url = format!("{base}/{name}");
        let out = dest.join(&name);
        eprintln!("  downloading {name}");
        http_get(&url, &out)?;
        if is_lfs_pointer(&out) {
            let _ = fs::remove_file(&out);
            return Err(format!("{name} came back as a Git LFS pointer, not the weights"));
        }
    }
    Ok(())
}

fn venv_python(venv: &Path) -> PathBuf {
    if cfg!(windows) { venv.join("Scripts").join("python.exe") } else { venv.join("bin").join("python") }
}

fn venv_pip(venv: &Path) -> PathBuf {
    if cfg!(windows) { venv.join("Scripts").join("pip.exe") } else { venv.join("bin").join("pip") }
}

fn install_python(data: &Path) -> Result<PathBuf, String> {
    let python = python_bin().ok_or("Python is not installed (python3 on Linux and macOS, python on Windows)")?;
    let venv = data.join("venv");
    let status = Command::new(&python).args(["-m", "venv"]).arg(&venv).status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("python -m venv failed (on Debian/Ubuntu install python3-venv)".into()); }
    let pip = venv_pip(&venv);
    let req = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("forge/requirements.txt");
    if req.is_file() {
        let status = Command::new(&pip).args(["install", "-r"]).arg(&req).status().map_err(|e| e.to_string())?;
        if !status.success() { return Err("pip install of the Forge environment failed".into()); }
    }
    let py = venv_python(&venv);
    if !py.is_file() { return Err(format!("venv did not create {}", py.display())); }
    Ok(py)
}

fn ask(prompt: &str) -> String {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    line.trim().to_string()
}

struct Flags {
    role: Option<String>,
    games: Option<Vec<String>>,
    download: bool,
    install_python: bool,
    yes: bool,
    lc_script: Option<String>,
}

fn flags(args: &[String]) -> Flags {
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let has = |k: &str| args.iter().any(|a| a == k);
    let games = opt("--games").map(|g| g.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "none").collect());
    Flags { role: opt("--role"), games, download: has("--download"), install_python: has("--install-python"), yes: has("--yes"), lc_script: opt("--lc-script") }
}

fn print_machine() {
    let gpu = if have("nvidia-smi") { command_out("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]).unwrap_or_else(|| "NVIDIA GPU".into()) } else { "no NVIDIA GPU detected".into() };
    let py = python_bin().and_then(|p| command_out(&p.to_string_lossy(), &["--version"])).unwrap_or_else(|| "not installed".into());
    let fetch = if have("curl") { "curl" } else if python_bin().is_some() { "python (no curl)" } else { "missing (install curl or Python)" };
    println!("system:  {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("gpu:     {gpu}");
    println!("python:  {py}");
    println!("fetch:   {fetch}");
    println!("docker:  {} (optional; Docker Desktop on Windows and macOS)", if have("docker") { "installed" } else { "not installed" });
    println!("data:    {}", data_dir().display());
    match std::env::consts::OS {
        "linux" => println!("note:    server and Link are the same binary. Forge uses an NVIDIA GPU when one is present."),
        "windows" => println!("note:    same commands as Linux. Allow signet.exe on localhost if the firewall asks. Forge uses an NVIDIA GPU."),
        "macos" => println!("note:    server and Link match Linux. The decision model needs an NVIDIA GPU, which Macs do not have; doors use the neutral stand-in until a Forge with that GPU answers."),
        _ => {}
    }
}

fn print_plan(role: &str) {
    println!();
    println!("Server uses this binary and the world file. Nothing else is downloaded.");
    if role == "server" {
        println!("Client pieces (model, Python, Forge) are skipped.");
        return;
    }
    println!("Client means Link and Forge. Heavy pieces, each optional until you ask:");
    println!("  model     about 1.8 GB, GitHub    signet setup --role client --download");
    println!("  python    PyTorch and Forge deps  signet setup --role client --install-python");
    println!("  minecraft a proto that speaks the bridge on 127.0.0.1:7790 (not in this install)");
    println!("  gmod      addon in gmod/signet next to this source, on a dedicated server you run");
    println!("  lethal    a host script from your own LAN capture (--lc-script); never downloaded");
}

/// Python and the Forge screen from the saved config, then the environment.
pub fn forge_runtime() -> Result<(String, String), String> {
    let c = load();
    let python = std::env::var("SIGNET_PYTHON").ok().filter(|s| !s.is_empty()).or_else(|| if c.python.is_empty() { None } else { Some(c.python) });
    let forge = std::env::var("SIGNET_FORGE").ok().filter(|s| !s.is_empty()).or_else(|| if c.forge.is_empty() { None } else { Some(c.forge) });
    match (python, forge) {
        (Some(p), Some(f)) if Path::new(&p).is_file() && Path::new(&f).is_file() => Ok((p, f)),
        _ => Err(format!("Forge is not set up. Run `signet setup --role client --download` and point it at a Python and signet_forge.py (data: {})", data_dir().display())),
    }
}

pub fn run(args: &[String]) -> i32 {
    let f = flags(args);
    println!("Signet setup");
    print_machine();
    let mut role = f.role.clone().unwrap_or_default();
    if role.is_empty() {
        if !io::stdin().is_terminal() {
            eprintln!("no role given. Pass --role server, --role client, or --role both.");
            return 2;
        }
        role = ask("Role? server (world only) / client (Link and Forge) / both: ");
    }
    if !matches!(role.as_str(), "server" | "client" | "both") {
        eprintln!("role must be server, client, or both");
        return 2;
    }
    let mut games = f.games.clone().unwrap_or_default();
    if games.is_empty() && role != "server" && io::stdin().is_terminal() && f.games.is_none() {
        let line = ask("Games for this PC? minecraft,gmod,lethal or none: ");
        if line != "none" { games = line.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(); }
    }
    for g in &games {
        if !matches!(g.as_str(), "minecraft" | "gmod" | "lethal") {
            eprintln!("unknown game '{g}'. Expected minecraft, gmod, or lethal.");
            return 2;
        }
    }
    print_plan(&role);

    let mut cfg = load();
    cfg.role = role.clone();
    cfg.games = games;
    if let Some(s) = &f.lc_script { cfg.lc_script = s.clone(); }
    let dest = data_dir().join("models").join(MODEL_NAME);
    let weights = dest.join("model.safetensors-00001-of-00001.safetensors");
    let client = role != "server";
    if client {
        let ready = weights.is_file() && !is_lfs_pointer(&weights);
        if ready {
            println!("model:   already present at {}", dest.display());
            cfg.model = dest.to_string_lossy().into();
        } else if f.download || (io::stdin().is_terminal() && !f.yes && ask("Download the decision model now? [y/N] ") == "y") {
            if let Some(src) = bundled_model() {
                println!("model:   copying from {}", src.display());
                if let Err(e) = copy_dir(&src, &dest) { eprintln!("{e}"); return 1; }
            } else if let Err(e) = download_model(&dest) {
                eprintln!("{e}");
                return 1;
            }
            cfg.model = dest.to_string_lossy().into();
            println!("model:   {}", dest.display());
        } else {
            println!("model:   not downloaded. Add --download when you want it.");
        }
        if f.install_python {
            match install_python(&data_dir()) {
                Ok(py) => { cfg.python = py.to_string_lossy().into(); println!("python:  {}", cfg.python); }
                Err(e) => { eprintln!("{e}"); return 1; }
            }
        } else if cfg.python.is_empty() {
            println!("python:  not installed into the data directory. Add --install-python when you want the Forge environment.");
        }
    }
    if save(&cfg).is_err() { eprintln!("cannot write {}", config_path().display()); return 1; }
    println!("saved:   {}", config_path().display());
    if client && cfg.forge.is_empty() {
        println!("Forge screen: set SIGNET_FORGE to signet_forge.py, or put that path in the config as forge=.");
        println!("The model alone does not start the screen. Link runs without it and draws neutral stand-ins.");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_and_config_roundtrip() {
        let dir = std::env::temp_dir().join(format!("signet-setup-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pointer = dir.join("w");
        fs::write(&pointer, "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 1\n").unwrap();
        assert!(is_lfs_pointer(&pointer));
        fs::write(dir.join("real"), b"not a pointer").unwrap();
        assert!(!is_lfs_pointer(&dir.join("real")));
        std::env::set_var("SIGNET_HOME", &dir);
        let c = Config { role: "client".into(), games: vec!["gmod".into()], model: "m".into(), python: String::new(), forge: String::new(), lc_script: String::new() };
        save(&c).unwrap();
        let got = load();
        assert_eq!(got.role, "client");
        assert_eq!(got.games, vec!["gmod".to_string()]);
        assert_eq!(got.model, "m");
        assert_eq!(data_dir_in("linux", "", "", "/home/a", "", ""), PathBuf::from("/home/a/.local/share/signet"));
        assert_eq!(data_dir_in("macos", "", "", "/Users/a", "", ""), PathBuf::from("/Users/a/Library/Application Support/signet"));
        assert_eq!(data_dir_in("windows", "", "", "", "C:/Users/a", "C:/Users/a/AppData/Local"), PathBuf::from("C:/Users/a/AppData/Local/signet"));
        assert_eq!(data_dir_in("windows", "D:/signet", "", "", "", ""), PathBuf::from("D:/signet"));
        assert_eq!(venv_python(Path::new("v")), if cfg!(windows) { PathBuf::from("v/Scripts/python.exe") } else { PathBuf::from("v/bin/python") });
        let _ = fs::remove_dir_all(&dir);
    }
}
