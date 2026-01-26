use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use super::osc_client::OscClient;
use crate::state::{ModuleType, Param, ParamValue};

pub type ModuleId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Connected,
    Error,
}

pub struct AudioEngine {
    client: Option<OscClient>,
    node_map: HashMap<ModuleId, i32>,
    next_node_id: i32,
    is_running: bool,
    scsynth_process: Option<Child>,
    server_status: ServerStatus,
    compile_receiver: Option<Receiver<Result<String, String>>>,
    is_compiling: bool,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            client: None,
            node_map: HashMap::new(),
            next_node_id: 1000,
            is_running: false,
            scsynth_process: None,
            server_status: ServerStatus::Stopped,
            compile_receiver: None,
            is_compiling: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn status(&self) -> ServerStatus {
        self.server_status
    }

    pub fn server_running(&self) -> bool {
        self.scsynth_process.is_some()
    }

    pub fn is_compiling(&self) -> bool {
        self.is_compiling
    }

    /// Start the scsynth server process
    pub fn start_server(&mut self) -> Result<(), String> {
        if self.scsynth_process.is_some() {
            return Err("Server already running".to_string());
        }

        self.server_status = ServerStatus::Starting;

        // Try to find scsynth in common locations
        let scsynth_paths = [
            "scsynth",
            "/Applications/SuperCollider.app/Contents/Resources/scsynth",
            "/usr/local/bin/scsynth",
            "/usr/bin/scsynth",
        ];

        let mut child = None;
        for path in &scsynth_paths {
            match Command::new(path)
                .args(["-u", "57110"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => {
                    child = Some(c);
                    break;
                }
                Err(_) => continue,
            }
        }

        match child {
            Some(c) => {
                self.scsynth_process = Some(c);
                self.server_status = ServerStatus::Running;
                // Give server time to start
                thread::sleep(Duration::from_millis(500));
                Ok(())
            }
            None => {
                self.server_status = ServerStatus::Error;
                Err("Could not find scsynth. Install SuperCollider.".to_string())
            }
        }
    }

    /// Stop the scsynth server process
    pub fn stop_server(&mut self) {
        // Disconnect first
        self.disconnect();

        if let Some(mut child) = self.scsynth_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.server_status = ServerStatus::Stopped;
    }

    /// Start compiling synthdefs in background thread
    pub fn compile_synthdefs_async(&mut self, scd_path: &Path) -> Result<(), String> {
        if self.is_compiling {
            return Err("Compilation already in progress".to_string());
        }

        if !scd_path.exists() {
            return Err(format!("File not found: {}", scd_path.display()));
        }

        let path = scd_path.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.compile_receiver = Some(rx);
        self.is_compiling = true;

        thread::spawn(move || {
            let result = Self::run_sclang(&path);
            let _ = tx.send(result);
        });

        Ok(())
    }

    /// Poll for compilation result (non-blocking)
    pub fn poll_compile_result(&mut self) -> Option<Result<String, String>> {
        if let Some(ref rx) = self.compile_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.compile_receiver = None;
                    self.is_compiling = false;
                    Some(result)
                }
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.compile_receiver = None;
                    self.is_compiling = false;
                    Some(Err("Compilation thread terminated unexpectedly".to_string()))
                }
            }
        } else {
            None
        }
    }

    /// Run sclang synchronously (called from background thread)
    fn run_sclang(scd_path: &PathBuf) -> Result<String, String> {
        // Try to find sclang in common locations
        let sclang_paths = [
            "sclang",
            "/Applications/SuperCollider.app/Contents/MacOS/sclang",
            "/usr/local/bin/sclang",
            "/usr/bin/sclang",
        ];

        for path in &sclang_paths {
            match Command::new(path).arg(scd_path).output() {
                Ok(output) => {
                    if output.status.success() {
                        return Ok("Synthdefs compiled successfully".to_string());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(format!("Compilation failed: {}", stderr));
                    }
                }
                Err(_) => continue,
            }
        }

        Err("Could not find sclang. Install SuperCollider.".to_string())
    }

    pub fn connect(&mut self, server_addr: &str) -> std::io::Result<()> {
        let client = OscClient::new(server_addr)?;
        self.client = Some(client);
        self.is_running = true;
        self.server_status = ServerStatus::Connected;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(ref client) = self.client {
            for &node_id in self.node_map.values() {
                let _ = client.free_node(node_id);
            }
        }
        self.node_map.clear();
        self.client = None;
        self.is_running = false;
        // Keep server_status as Running if scsynth is still running
        if self.scsynth_process.is_some() {
            self.server_status = ServerStatus::Running;
        } else {
            self.server_status = ServerStatus::Stopped;
        }
    }

    fn synth_def_name(module_type: ModuleType) -> &'static str {
        match module_type {
            ModuleType::SawOsc => "tuidaw_saw",
            ModuleType::SinOsc => "tuidaw_sin",
            ModuleType::SqrOsc => "tuidaw_sqr",
            ModuleType::TriOsc => "tuidaw_tri",
            ModuleType::Lpf => "tuidaw_lpf",
            ModuleType::Hpf => "tuidaw_hpf",
            ModuleType::Bpf => "tuidaw_bpf",
            ModuleType::AdsrEnv => "tuidaw_adsr",
            ModuleType::Lfo => "tuidaw_lfo",
            ModuleType::Delay => "tuidaw_delay",
            ModuleType::Reverb => "tuidaw_reverb",
            ModuleType::Output => "tuidaw_output",
        }
    }

    pub fn create_synth(
        &mut self,
        module_id: ModuleId,
        module_type: ModuleType,
        params: &[Param],
    ) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected")?;

        let node_id = self.next_node_id;
        self.next_node_id += 1;

        let param_pairs: Vec<(String, f32)> = params
            .iter()
            .filter_map(|p| match &p.value {
                ParamValue::Float(v) => Some((p.name.clone(), *v)),
                ParamValue::Int(v) => Some((p.name.clone(), *v as f32)),
                ParamValue::Bool(v) => Some((p.name.clone(), if *v { 1.0 } else { 0.0 })),
            })
            .collect();

        client
            .create_synth(Self::synth_def_name(module_type), node_id, &param_pairs)
            .map_err(|e| e.to_string())?;

        self.node_map.insert(module_id, node_id);
        Ok(())
    }

    pub fn free_synth(&mut self, module_id: ModuleId) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected")?;
        if let Some(node_id) = self.node_map.remove(&module_id) {
            client.free_node(node_id).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn set_param(&self, module_id: ModuleId, param: &str, value: f32) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected")?;
        let node_id = self
            .node_map
            .get(&module_id)
            .ok_or_else(|| format!("No synth for module {}", module_id))?;
        client
            .set_param(*node_id, param, value)
            .map_err(|e| e.to_string())
    }

    pub fn load_synthdefs(&self, dir: &Path) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected")?;

        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().map_or(false, |e| e == "scsyndef") {
                let data = fs::read(&path).map_err(|e| e.to_string())?;
                client
                    .send_message("/d_recv", vec![rosc::OscType::Blob(data)])
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
